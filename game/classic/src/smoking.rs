//! Cigarette packets, matches, lighters and cigarettes. A lit match, lighter or cigarette
//! is a small flame: it touches the air where it is at 700 K every life cycle, enough to
//! light plasma. Spec: `docs/reference/smoking.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::environment::LIFE_TICKS;
use crate::equipment::{Equipment, WornBy};
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Zone};
use crate::rng::GameRng;
use crate::time::WorldTime;

/// How hot a small flame is, K (Luna `hotspot_expose(700, 5)`).
pub const FLAME_HEAT: f64 = 700.0;
/// Cigarettes in a new packet (Luna `cigcount`).
pub const PACKET_CIGARETTES: u8 = 6;
/// Life cycles a match burns (Luna `smoketime = 15`).
pub const MATCH_CYCLES: u16 = 15;
/// Life cycles a cigarette burns (Luna `smoketime = 150`).
pub const CIGARETTE_CYCLES: u16 = 150;
/// Chance a cheap lighter lights without burning you, percent (Luna 80).
pub const LIGHTER_SAFE: f32 = 80.0;
/// Burn to an arm when it doesn't (Luna 2).
pub const LIGHTER_BURN: f32 = 2.0;

/// A packet of cigarettes: how many are left.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CigarettePacket(pub u8);

/// A match: unlit, burning (cycles left), or burnt out. Dropped while lit, it looks burnt
/// at once but burns on until its time is up (Luna `match/dropped`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    Unlit,
    Lit(u16),
    Burnt(u16),
    Spent,
}

/// A lighter: its colour, and whether it is lit.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lighter {
    pub colour: &'static str,
    pub lit: bool,
}

/// A cigarette: lit with how many cycles left, or not.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cigarette {
    pub lit: Option<u16>,
}

/// The smoking part a new item of prototype `proto` has. `roll` (0 to 3) picks a lighter's
/// colour (Luna `lighter/random`).
pub fn spawn_smokable(entity: &mut EntityWorldMut, proto: &str, roll: i32) {
    match proto {
        "cigarette-packet" => {
            entity.insert(CigarettePacket(PACKET_CIGARETTES));
        }
        "match" => {
            entity.insert(Match::Unlit);
        }
        "lighter" => {
            let colour = ["r", "c", "y", "g"][roll.clamp(0, 3) as usize];
            entity.insert(Lighter { colour, lit: false });
        }
        "cigarette" => {
            entity.insert(Cigarette { lit: None });
        }
        _ => {}
    }
}

/// Everything clicks with smoking things need.
#[derive(SystemParam)]
pub struct SmokingWork<'w, 's> {
    packets: Query<'w, 's, &'static mut CigarettePacket>,
    matches: Query<'w, 's, &'static mut Match>,
    lighters: Query<'w, 's, &'static mut Lighter>,
    cigarettes: Query<'w, 's, &'static mut Cigarette>,
    looks: Query<'w, 's, &'static Appearance>,
    equipment: Query<'w, 's, &'static Equipment>,
    bodies: Query<'w, 's, &'static mut Body>,
}

/// What a lit tool lights a cigarette with, for the message (Luna `cigarette/attackby`).
enum Flame {
    Welder,
    Lighter,
    Other,
}

impl SmokingWork<'_, '_> {
    fn sprite(&self, e: Entity) -> String {
        self.looks.get(e).map(|a| a.sprite.clone()).unwrap_or_default()
    }

    /// Whether `item` is lit and can light a cigarette. `welder_lit`: it is a lit welding
    /// tool (`welding.rs` knows).
    fn flame(&self, item: Entity, welder_lit: bool) -> Option<Flame> {
        if welder_lit {
            Some(Flame::Welder)
        } else if self.lighters.get(item).is_ok_and(|l| l.lit) {
            Some(Flame::Lighter)
        } else if matches!(self.matches.get(item), Ok(Match::Lit(_)))
            || self.cigarettes.get(item).is_ok_and(|c| c.lit.is_some())
        {
            Some(Flame::Other)
        } else {
            None
        }
    }

    /// Whether this click is one of ours: a match struck on a matchbox, a cigarette lit, a
    /// cigarette taken out of the packet in the other hand.
    pub fn takes(
        &self,
        mob: Entity,
        hands: &Hands,
        target: Entity,
        matchbox: bool,
        welder_lit: bool,
    ) -> bool {
        let active = hands.active_item();
        match active {
            Some(item) if matchbox => matches!(self.matches.get(item), Ok(Match::Unlit)),
            Some(item) => {
                self.flame(item, welder_lit).is_some()
                    && (self.cigarettes.get(target).is_ok_and(|c| c.lit.is_none())
                        || (target == mob && self.worn_cigarette(mob).is_some()))
            }
            // Only from the other hand; from the floor it is picked up.
            None => {
                self.packets.contains(target) && (hands.left == Some(target) || hands.right == Some(target))
            }
        }
    }

    fn worn_cigarette(&self, mob: Entity) -> Option<Entity> {
        let mask = self
            .equipment
            .get(mob)
            .ok()?
            .0
            .get(&rs13_world::Slot::Mask)
            .copied()?;
        self.cigarettes
            .get(mask)
            .ok()
            .filter(|c| c.lit.is_none())
            .map(|_| mask)
    }

    /// Do the click [`takes`](Self::takes) said is ours. Returns the messages.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        target: Entity,
        welder_lit: bool,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let Some(item) = hands.active_item() else {
            // Luna `cigpacket/attack_hand`: one into the empty hand.
            let mut packet = self.packets.get_mut(target).ok()?;
            if packet.0 == 0 {
                return Some(ClickOutcome::told("cigarettes-out"));
            }
            packet.0 -= 1;
            let hand = hands.active;
            commands.queue(move |world: &mut World| {
                let at = world.get::<GridPos>(mob).map_or(TilePos::new(0, 0, 0), |p| p.0);
                let Ok(cig) = crate::objects::spawn_object(world, &ObjectSpec::new("cigarette"), at) else {
                    return;
                };
                world.entity_mut(cig).remove::<GridPos>().insert(HeldBy(mob));
                if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                    *hands.slot_mut(hand) = Some(cig);
                }
            });
            return Some(ClickOutcome::default());
        };
        if let Ok(mut m) = self.matches.get_mut(item)
            && *m == Match::Unlit
        {
            // Struck on the matchbox: no message (Luna `matches/attackby`).
            *m = Match::Lit(MATCH_CYCLES);
            return Some(ClickOutcome::default());
        }
        let flame = self.flame(item, welder_lit)?;
        let cig = if target == mob {
            self.worn_cigarette(mob)?
        } else {
            target
        };
        let mut c = self.cigarettes.get_mut(cig).ok()?;
        if c.lit.is_some() {
            return None;
        }
        c.lit = Some(CIGARETTE_CYCLES);
        let id = match flame {
            Flame::Welder => "cigarette-lit-welder",
            Flame::Lighter => "cigarette-lit-lighter",
            Flame::Other => "cigarette-lit",
        };
        Some(ClickOutcome {
            to_mob: None,
            seen: Some(
                LocMsg::new(id)
                    .arg("item", "cigarette")
                    .arg("tool", self.sprite(item)),
            ),
        })
    }

    /// Whether `item` does something used in the hand.
    pub fn uses_self(&self, item: Entity) -> bool {
        self.lighters.contains(item) || self.cigarettes.get(item).is_ok_and(|c| c.lit.is_some())
    }

    /// Luna `lighter/attack_self`: light it (a cheap one may burn you) or shut it; a lit
    /// cigarette is dropped and trodden out, leaving a butt.
    pub fn use_self(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        hands: &mut Hands,
        item: Entity,
        rng: &mut GameRng,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        if let Ok(mut lighter) = self.lighters.get_mut(item) {
            if lighter.lit {
                lighter.lit = false;
                return Some(ClickOutcome {
                    to_mob: None,
                    seen: Some(LocMsg::new("lighter-off").arg("item", "lighter")),
                });
            }
            lighter.lit = true;
            if rng.prob(LIGHTER_SAFE) {
                return Some(ClickOutcome {
                    to_mob: None,
                    seen: Some(LocMsg::new("lighter-on").arg("item", "lighter")),
                });
            }
            let arm = if rng.prob(50.0) { Zone::LArm } else { Zone::RArm };
            if let Ok(mut body) = self.bodies.get_mut(mob) {
                body.take_damage(arm, 0.0, LIGHTER_BURN);
            }
            return Some(ClickOutcome {
                to_mob: Some(LocMsg::new("lighter-burn-self")),
                seen: Some(LocMsg::new("lighter-on-burnt").arg("item", "lighter")),
            });
        }
        let c = self.cigarettes.get(item).ok()?;
        c.lit?;
        *hands.slot_mut(hands.active) = None;
        commands.entity(item).despawn();
        commands.queue(move |world: &mut World| {
            let _ = crate::objects::spawn_object(world, &ObjectSpec::new("cigarette-butt"), mob_pos);
        });
        Some(ClickOutcome {
            to_mob: None,
            seen: Some(LocMsg::new("cigarette-trodden").arg("item", "cigarette")),
        })
    }

    /// What examining a packet adds (Luna's `desc` once it has been opened).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let packet = self.packets.get(target).ok()?;
        (packet.0 < PACKET_CIGARETTES)
            .then(|| LocMsg::new("examine-cigarettes").num("count", i64::from(packet.0)))
    }
}

type Where<'a> = (Option<&'a GridPos>, Option<&'a HeldBy>, Option<&'a WornBy>);

/// Where something is: on the floor, or on whoever holds or wears it.
fn place(at: Where, places: &Query<&GridPos>) -> Option<TilePos> {
    match at {
        (Some(p), ..) => Some(p.0),
        (_, Some(h), _) => places.get(h.0).ok().map(|p| p.0),
        (_, _, Some(w)) => places.get(w.0).ok().map(|p| p.0),
        _ => None,
    }
}

/// Every life cycle, every small flame touches the air where it is; matches and cigarettes
/// burn down (Luna `process`). A burnt-out cigarette leaves a butt on the floor and tells
/// whoever had it: "Your cigarette goes out."
#[allow(clippy::type_complexity)]
pub fn burn_flames(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    places: Query<&GridPos>,
    mut matches: Query<(&mut Match, Option<&GridPos>, Option<&HeldBy>, Option<&WornBy>)>,
    lighters: Query<(&Lighter, Option<&GridPos>, Option<&HeldBy>, Option<&WornBy>)>,
    mut cigarettes: Query<(
        Entity,
        &mut Cigarette,
        Option<&GridPos>,
        Option<&HeldBy>,
        Option<&WornBy>,
    )>,
    mut owners: Query<(&mut Hands, &mut Equipment, &mut crate::actions::Outbox)>,
    mut commands: Commands,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(LIFE_TICKS) {
        return;
    }
    let mut hot: Vec<TilePos> = Vec::new();
    for (mut m, pos, held, worn) in &mut matches {
        let at = place((pos, held, worn), &places);
        let (left, dropped) = match *m {
            Match::Lit(left) => (left, false),
            Match::Burnt(left) => (left, true),
            _ => continue,
        };
        if left <= 1 {
            *m = Match::Spent;
            continue;
        }
        // Lit and lying on the floor: burnt to look at, still burning.
        *m = if dropped || pos.is_some() {
            Match::Burnt(left - 1)
        } else {
            Match::Lit(left - 1)
        };
        hot.extend(at);
    }
    for (lighter, pos, held, worn) in &lighters {
        if lighter.lit {
            hot.extend(place((pos, held, worn), &places));
        }
    }
    for (entity, mut cig, pos, held, worn) in &mut cigarettes {
        let Some(left) = cig.lit else { continue };
        let at = place((pos, held, worn), &places);
        if left > 1 {
            cig.lit = Some(left - 1);
            hot.extend(at);
            continue;
        }
        // Burnt out: a butt where it was, and the smoker is told.
        let owner = held.map(|h| h.0).or(worn.map(|w| w.0));
        if let Some(owner) = owner
            && let Ok((mut hands, mut equipment, mut outbox)) = owners.get_mut(owner)
        {
            for hand in [rs13_world::Hand::Left, rs13_world::Hand::Right] {
                if hands.get(hand) == Some(entity) {
                    *hands.slot_mut(hand) = None;
                }
            }
            equipment.0.retain(|_, e| *e != entity);
            outbox
                .0
                .push(LocMsg::new("cigarette-out").arg("item", "cigarette"));
        }
        commands.entity(entity).despawn();
        if let Some(at) = at {
            commands.queue(move |world: &mut World| {
                let _ = crate::objects::spawn_object(world, &ObjectSpec::new("cigarette-butt"), at);
            });
        }
    }
    for at in hot {
        atmos.0.expose(at, FLAME_HEAT);
    }
}

/// Sprites: a packet by what is left once opened, a match lit or burnt, a lighter by
/// colour and lit, a cigarette lit.
#[allow(clippy::type_complexity)]
pub fn update_smoking_looks(
    mut looks: ParamSet<(
        Query<(&CigarettePacket, &mut Appearance), Changed<CigarettePacket>>,
        Query<(&Match, &mut Appearance), Changed<Match>>,
        Query<(&Lighter, &mut Appearance), Changed<Lighter>>,
        Query<(&Cigarette, &mut Appearance), Changed<Cigarette>>,
    )>,
) {
    let set = |look: &mut Appearance, state: String| {
        if look.state != state {
            look.state = state;
        }
    };
    for (packet, mut look) in &mut looks.p0() {
        let state = if packet.0 >= PACKET_CIGARETTES {
            String::new()
        } else {
            packet.0.to_string()
        };
        set(&mut look, state);
    }
    for (m, mut look) in &mut looks.p1() {
        let state = match m {
            Match::Unlit => "",
            Match::Lit(_) => "lit",
            Match::Burnt(_) | Match::Spent => "burnt",
        };
        set(&mut look, state.to_owned());
    }
    for (lighter, mut look) in &mut looks.p2() {
        let on = if lighter.lit { "-on" } else { "" };
        set(&mut look, format!("{}{on}", lighter.colour));
    }
    for (cig, mut look) in &mut looks.p3() {
        set(&mut look, if cig.lit.is_some() { "lit" } else { "" }.to_owned());
    }
}

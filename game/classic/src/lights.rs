//! Lights: light fixtures, the tubes and bulbs in them, and light switches.
//! Spec: `docs/reference/lights.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, GridPos, Hand, LightSource, ObjectSpec, Slot, TilePos};

use crate::actions::ClickOutcome;
use crate::equipment::Equipment;
use crate::hands::{Conducts, Hands, HeldBy, Item};
use crate::health::{Body, Zone};
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::wires::PendingShocks;

/// Watts a lit fixture draws per unit of brightness (Luna `LIGHTING_POWER_FACTOR`).
pub const LIGHTING_POWER_FACTOR: f64 = 25.0;
/// Luna `burnchance`: each switch on or off may burn the light out, with a chance in
/// percent of this times the square of the number of switchings, up to 60.
pub const BURN_CHANCE: f64 = 0.001;
/// Chance in percent that a small light starts broken (Luna `light/small/New`).
pub const SMALL_BROKEN_CHANCE: f32 = 5.0;
/// Chance in percent that sticking metal into a live socket shocks you.
pub const SOCKET_SHOCK_CHANCE: f32 = 75.0;
/// Chance in percent that smashing a lit light with metal shocks you.
pub const SMASH_SHOCK_CHANCE: f32 = 50.0;

/// What a fixture takes: a tube, or a bulb in a small light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fitting {
    Tube,
    Bulb,
}

impl Fitting {
    /// The item prototype that fits it.
    pub fn item(self) -> &'static str {
        match self {
            Fitting::Tube => "light-tube",
            Fitting::Bulb => "light-bulb",
        }
    }

    /// The fixture sprite states start with this.
    fn base(self) -> &'static str {
        match self {
            Fitting::Tube => "tube",
            Fitting::Bulb => "bulb",
        }
    }

    /// Luna's brightness: a tube 8, a bulb 5.
    pub fn brightness(self) -> f64 {
        match self {
            Fitting::Tube => 8.0,
            Fitting::Bulb => 5.0,
        }
    }

    fn of_item(sprite: &str) -> Option<Self> {
        match sprite {
            "light-tube" => Some(Fitting::Tube),
            "light-bulb" => Some(Fitting::Bulb),
            _ => None,
        }
    }
}

/// The tube or bulb in a fixture, or the one in the hand (Luna `LIGHT_OK` …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightStatus {
    Ok,
    Empty,
    Burned,
    Broken,
}

impl LightStatus {
    /// A tube or bulb item shows its status as its sprite state.
    pub fn item_state(self) -> &'static str {
        match self {
            LightStatus::Burned => "burned",
            LightStatus::Broken => "broken",
            _ => "",
        }
    }

    fn of_item(state: &str) -> Self {
        match state {
            "burned" => LightStatus::Burned,
            "broken" => LightStatus::Broken,
            _ => LightStatus::Ok,
        }
    }
}

/// A light fixture on a wall. It is lit while it has a working tube or bulb, its area's
/// lighting is powered and the area's light switch is on.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub fitting: Fitting,
    pub status: LightStatus,
    pub on: bool,
    /// Times it went on or off since this tube went in (Luna `switchcount`).
    pub switches: u32,
    pub facing: Dir,
    /// Red, green and blue strength while lit, in tiles of reach (Luna `brightnessred`
    /// …).
    pub color: [u8; 3],
}

/// A light switch: turns every light in its area on or off.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightSwitch;

/// Each area's light switch setting (Luna `area.lightswitch`). Areas start lit.
#[derive(Resource, Debug, Default)]
pub struct AreaLights(pub HashMap<String, bool>);

impl AreaLights {
    pub fn is_on(&self, area: Option<&str>) -> bool {
        area.is_none_or(|a| self.0.get(a).copied().unwrap_or(true))
    }
}

/// Prototype ids of light fixtures and switches.
pub fn is_light_proto(proto: &str) -> bool {
    proto == "light-switch" || fixture_kind(proto).is_some()
}

/// A fixture prototype's fitting and colour (Luna `brightnessred`, `brightnessgreen`,
/// `brightnessblue`). The coloured ones look like the plain ones.
pub fn fixture_kind(proto: &str) -> Option<(Fitting, [u8; 3])> {
    Some(match proto {
        "light" => (Fitting::Tube, [8, 8, 8]),
        "light-blue" => (Fitting::Tube, [5, 5, 7]),
        "light-red" => (Fitting::Tube, [8, 2, 2]),
        "light-small" => (Fitting::Bulb, [5, 5, 4]),
        "light-small-red" => (Fitting::Bulb, [5, 0, 0]),
        _ => return None,
    })
}

/// Spawn a light fixture or a light switch. Fixtures face the wall they hang on (Luna's
/// default is south).
pub fn spawn_light(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: rs13_world::NetId) -> Entity {
    let proto = spec.id.as_str();
    let power = PowerUser::new(Channel::Lighting, 0.0);
    if proto == "light-switch" {
        return world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(proto, "light1"),
                LightSwitch,
                power,
            ))
            .id();
    }
    let (fitting, color) = fixture_kind(proto).unwrap_or((Fitting::Tube, [8, 8, 8]));
    let sprite = match fitting {
        Fitting::Tube => "light",
        Fitting::Bulb => "light-small",
    };
    world.init_resource::<GameRng>();
    let broken = fitting == Fitting::Bulb && world.resource_mut::<GameRng>().prob(SMALL_BROKEN_CHANCE);
    let light = Light {
        fitting,
        status: if broken {
            LightStatus::Broken
        } else {
            LightStatus::Ok
        },
        on: false,
        switches: 0,
        facing: spec.dir.unwrap_or(Dir::South),
        color,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(sprite, light_look(&light)),
            light,
            power,
        ))
        .id()
}

/// A fixture's sprite state: `tube1-north`, `bulb-broken-east`.
fn light_look(light: &Light) -> String {
    let base = light.fitting.base();
    let status = match (light.status, light.on) {
        (LightStatus::Ok, true) => "1",
        (LightStatus::Ok, false) => "0",
        (LightStatus::Empty, _) => "-empty",
        (LightStatus::Burned, _) => "-burned",
        (LightStatus::Broken, _) => "-broken",
    };
    let facing = match light.facing {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    };
    format!("{base}{status}-{facing}")
}

/// Chance in percent that a light burns out on its `switches`th switching (Luna
/// `prob(min(60, switchcount * switchcount * burnchance))`).
pub fn burn_chance(switches: u32) -> f32 {
    (f64::from(switches).powi(2) * BURN_CHANCE).min(60.0) as f32
}

/// Turn each light on or off, and draw its power (Luna `light/update`, `power_change`
/// and `process`).
pub fn run_lights(
    maps: crate::decks::Decks,
    switches: Res<AreaLights>,
    mut rng: ResMut<GameRng>,
    mut lights: Query<(&GridPos, &mut Light, &mut PowerUser)>,
) {
    for (pos, mut light, mut power) in &mut lights {
        let area = maps.area(pos.0);
        let wanted = light.status == LightStatus::Ok && power.powered && switches.is_on(area);
        if wanted != light.on {
            let mut new = *light;
            new.on = wanted;
            new.switches += 1;
            if rng.prob(burn_chance(new.switches)) {
                new.status = LightStatus::Burned;
                new.on = false;
            }
            *light = new;
        }
        let watts = if light.on {
            light.fitting.brightness() * LIGHTING_POWER_FACTOR
        } else {
            0.0
        };
        if power.watts != watts {
            power.watts = watts;
        }
    }
}

/// Keep fixtures and switches looking as they are (Luna `update_icon`): a switch shows
/// `light-p` without power, else its area's setting.
pub fn update_light_looks(
    maps: crate::decks::Decks,
    switches: Res<AreaLights>,
    mut lights: Query<(&Light, &mut Appearance), Changed<Light>>,
    mut panels: Query<(&GridPos, &PowerUser, &mut Appearance), (With<LightSwitch>, Without<Light>)>,
) {
    for (light, mut look) in &mut lights {
        let state = light_look(light);
        if look.state != state {
            look.state = state;
        }
    }
    for (pos, power, mut look) in &mut panels {
        let state = if !power.powered {
            "light-p"
        } else if switches.is_on(maps.area(pos.0)) {
            "light1"
        } else {
            "light0"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

/// A lit fixture gives off its light; an unlit one none (Luna `ul_SetLuminosity`).
pub fn update_light_sources(
    mut commands: Commands,
    lights: Query<(Entity, &Light, Option<&LightSource>)>,
    changed: Query<(), Changed<Light>>,
    alert: Res<crate::escape::StationAlert>,
) {
    for (entity, light, source) in &lights {
        if !alert.is_changed() && !changed.contains(entity) {
            continue;
        }
        // Red alert turns every light red (Luna's `isalert`), as bright as it was.
        let colour = if alert.red {
            [light.color.into_iter().max().unwrap_or(0), 0, 0]
        } else {
            light.color
        };
        let wanted = light.on.then_some(LightSource(colour));
        if source.copied() == wanted {
            continue;
        }
        match wanted {
            Some(source) => commands.entity(entity).insert(source),
            None => commands.entity(entity).remove::<LightSource>(),
        };
    }
}

/// Everything clicking lights and switches needs.
#[derive(SystemParam)]
pub struct LightWork<'w, 's> {
    lights: Query<'w, 's, (&'static GridPos, &'static mut Light)>,
    switches: Query<'w, 's, &'static GridPos, (With<LightSwitch>, Without<Light>)>,
    power: Query<'w, 's, &'static PowerUser>,
    maps: crate::decks::Decks<'w, 's>,
    area_lights: ResMut<'w, AreaLights>,
    looks: Query<'w, 's, &'static Appearance>,
    items: Query<'w, 's, &'static Item>,
    conducting: Query<'w, 's, (), With<Conducts>>,
    worn: Query<'w, 's, &'static Equipment>,
    /// The game's dice, shared with other clicks in `process_actions`.
    pub(crate) rng: ResMut<'w, GameRng>,
}

impl LightWork<'_, '_> {
    /// Whether `target` is a light fixture or a light switch.
    pub fn contains(&self, target: Entity) -> bool {
        self.lights.contains(target) || self.switches.contains(target)
    }

    fn area_on(&self, pos: TilePos) -> bool {
        let area = self.maps.area(pos);
        self.area_lights.is_on(area)
    }

    /// Clicking a light or a switch, with or without something in the hand. The caller
    /// checks reach. `harm` is the mob's intent.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        harm: bool,
        target: Entity,
        shocks: &mut PendingShocks,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        if let Ok(pos) = self.switches.get(target) {
            // Luna `light_switch/attack_hand`: flips its whole area, powered or not.
            let area = self.maps.area(pos.0).map(str::to_owned);
            if let Some(area) = area {
                let on = self.area_lights.is_on(Some(&area));
                self.area_lights.0.insert(area, !on);
            }
            return Some(ClickOutcome::default());
        }
        let (pos, light) = self.lights.get(target).ok()?;
        let (pos, light) = (pos.0, *light);
        let fitting = light.fitting.item();
        let powered = self.power.get(target).is_ok_and(|p| p.powered) && self.area_on(pos);

        let Some(held) = hands.active_item() else {
            return Some(self.take_out(mob, hands, harm, target, light, commands));
        };
        let sprite = self.looks.get(held).map(|a| a.sprite.clone()).unwrap_or_default();
        let state = self.looks.get(held).map(|a| a.state.clone()).unwrap_or_default();

        // A tube or bulb: into an empty fixture, or in place of a dead one.
        if let Some(kind) = Fitting::of_item(&sprite) {
            if !matches!(
                light.status,
                LightStatus::Empty | LightStatus::Burned | LightStatus::Broken
            ) {
                return Some(ClickOutcome::default());
            }
            if kind != light.fitting {
                return Some(told("light-wrong-type", fitting));
            }
            let id = if light.status == LightStatus::Empty {
                "light-inserted"
            } else {
                // The old one drops out.
                drop_light(light.fitting, light.status, pos, commands);
                "light-replaced"
            };
            let active = hands.active;
            *hands.slot_mut(active) = None;
            commands.entity(held).despawn();
            if let Ok((_, mut light)) = self.lights.get_mut(target) {
                light.status = LightStatus::of_item(&state);
                light.switches = 0;
            }
            return Some(told(id, fitting));
        }

        let conducts = self.conducting.contains(held);
        match light.status {
            // Anything else in an empty socket (Luna: a screwdriver opens the casing).
            LightStatus::Empty => {
                if powered && conducts {
                    shocks.0.push((mob, SOCKET_SHOCK_CHANCE));
                }
                Some(told("light-stick", &sprite))
            }
            LightStatus::Ok | LightStatus::Burned => {
                let force = self.items.get(held).map_or(0.0, |i| i.force);
                if !self.rng.prob((1.0 + force) * 12.0) {
                    return Some(ClickOutcome::told("light-hit"));
                }
                if light.on && conducts {
                    shocks.0.push((mob, SMASH_SHOCK_CHANCE));
                }
                if let Ok((_, mut light)) = self.lights.get_mut(target) {
                    light.status = LightStatus::Broken;
                }
                Some(ClickOutcome {
                    to_mob: Some(LocMsg::new("light-smashed")),
                    seen: Some(LocMsg::new("light-smashed-seen")),
                })
            }
            LightStatus::Broken => Some(ClickOutcome::default()),
        }
    }

    /// An empty hand takes the tube or bulb out. A lit one burns a bare hand (Luna
    /// `light/attack_hand`): with harm intent the hand smashes it instead.
    fn take_out(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        harm: bool,
        target: Entity,
        light: Light,
        commands: &mut Commands,
    ) -> ClickOutcome {
        let fitting = light.fitting.item();
        if light.status == LightStatus::Empty {
            return told("light-no-fitting", fitting);
        }
        let hand = hands.active;
        let zone = match hand {
            Hand::Left => Zone::LHand,
            Hand::Right => Zone::RHand,
        };
        let gloved = self.worn.get(mob).is_ok_and(|e| e.get(Slot::Gloves).is_some());
        if light.on && !gloved {
            let (brute, burn) = if harm { (2.0, 6.0) } else { (0.0, 5.0) };
            commands.queue(move |world: &mut World| {
                if let Some(mut body) = world.get_mut::<Body>(mob) {
                    body.take_damage(zone, brute, burn);
                }
            });
            if !harm {
                return told("light-burn-hand", fitting);
            }
            if let Ok((_, mut light)) = self.lights.get_mut(target) {
                light.status = LightStatus::Broken;
            }
            return told("light-smash-burn", fitting);
        }
        let status = light.status;
        let kind = light.fitting;
        if let Ok((pos, mut light)) = self.lights.get_mut(target) {
            light.status = LightStatus::Empty;
            light.switches = 0;
            let pos = pos.0;
            commands.queue(move |world: &mut World| {
                let Some(item) = spawn_light_item(world, kind, status, pos) else {
                    return;
                };
                // Into the hand that took it, if that is still free.
                let free = world.get::<Hands>(mob).is_some_and(|h| h.get(hand).is_none());
                if free {
                    world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
                    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                        *hands.slot_mut(hand) = Some(item);
                    }
                }
            });
        }
        told("light-removed", fitting)
    }

    /// What examining a light or switch adds (Luna `light/examine`,
    /// `light_switch/examine`).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if let Ok(pos) = self.switches.get(target) {
            let id = if self.area_on(pos.0) {
                "examine-light-on"
            } else {
                "examine-light-off"
            };
            return Some(LocMsg::new(id));
        }
        let (_, light) = self.lights.get(target).ok()?;
        let id = match (light.status, light.on) {
            (LightStatus::Ok, true) => "examine-light-on",
            (LightStatus::Ok, false) => "examine-light-off",
            (LightStatus::Empty, _) => "examine-light-empty",
            (LightStatus::Burned, _) => "examine-light-burned",
            (LightStatus::Broken, _) => "examine-light-broken",
        };
        Some(LocMsg::new(id).arg("item", light.fitting.item()))
    }
}

fn told(id: &str, item: &str) -> ClickOutcome {
    ClickOutcome {
        to_mob: Some(LocMsg::new(id).arg("item", item.to_owned())),
        seen: None,
    }
}

/// A tube or bulb with `status` on the floor at `pos`.
fn spawn_light_item(world: &mut World, kind: Fitting, status: LightStatus, pos: TilePos) -> Option<Entity> {
    let item = crate::objects::spawn_object(world, &ObjectSpec::new(kind.item()), pos).ok()?;
    if let Some(mut look) = world.get_mut::<Appearance>(item) {
        look.state = status.item_state().to_owned();
    }
    Some(item)
}

fn drop_light(kind: Fitting, status: LightStatus, pos: TilePos, commands: &mut Commands) {
    commands.queue(move |world: &mut World| {
        spawn_light_item(world, kind, status, pos);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_follow_luna_states() {
        let mut light = Light {
            fitting: Fitting::Tube,
            status: LightStatus::Ok,
            on: true,
            switches: 0,
            facing: Dir::North,
            color: [8, 8, 8],
        };
        assert_eq!(light_look(&light), "tube1-north");
        light.on = false;
        assert_eq!(light_look(&light), "tube0-north");
        light.fitting = Fitting::Bulb;
        light.status = LightStatus::Burned;
        light.facing = Dir::South;
        assert_eq!(light_look(&light), "bulb-burned-south");
    }

    #[test]
    fn coloured_fixtures_keep_luna_brightness() {
        assert_eq!(fixture_kind("light"), Some((Fitting::Tube, [8, 8, 8])));
        assert_eq!(fixture_kind("light-blue"), Some((Fitting::Tube, [5, 5, 7])));
        assert_eq!(fixture_kind("light-red"), Some((Fitting::Tube, [8, 2, 2])));
        assert_eq!(fixture_kind("light-small"), Some((Fitting::Bulb, [5, 5, 4])));
        assert_eq!(fixture_kind("light-small-red"), Some((Fitting::Bulb, [5, 0, 0])));
        assert!(is_light_proto("light-switch") && !is_light_proto("light-tube"));
    }

    #[test]
    fn burning_out_gets_likelier_with_use() {
        assert!(burn_chance(1) < 0.01);
        assert!((burn_chance(100) - 10.0).abs() < 1e-3);
        assert_eq!(burn_chance(1_000), 60.0);
    }
}

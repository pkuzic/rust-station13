//! Bedsheets and linen bins: a sheet spread over whoever lies under it, and a bin of them
//! that something small can be hidden in. Spec: `docs/reference/bedsheets.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::hands::{Hands, HeldBy, Item};
use crate::paperwork::Filed;

pub const BEDSHEET: &str = "bedsheet";
pub const LINEN_BIN: &str = "linen-bin";
/// Sheets in a new bin (Luna `amount`).
pub const BIN_SHEETS: u16 = 10;
/// The look of a sheet spread over what lies under it (Luna `layer = 5`).
pub const SPREAD: &str = "spread";

pub fn is_bedsheet(sprite: &str) -> bool {
    sprite == BEDSHEET || sprite.starts_with("bedsheet-")
}

/// A bedsheet: spread over what lies under it, or not.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bedsheet {
    pub spread: bool,
}

/// A linen bin: sheets left, the ones put back (handed out first), and something hidden
/// among them.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LinenBin {
    pub amount: u16,
    pub returned: Vec<Entity>,
    pub hidden: Option<Entity>,
}

pub fn spawn_bedsheet(entity: &mut EntityWorldMut, spec: &ObjectSpec) {
    if is_bedsheet(&spec.id) {
        entity.insert(Bedsheet::default());
    }
}

pub fn spawn_linen_bin(world: &mut World, pos: TilePos, id: rs13_world::NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(LINEN_BIN, "linenbin-full"),
            LinenBin {
                amount: BIN_SHEETS,
                returned: Vec::new(),
                hidden: None,
            },
        ))
        .id()
}

/// Everything clicks with bedsheets need.
#[derive(SystemParam)]
pub struct LinenWork<'w, 's> {
    bins: Query<'w, 's, &'static mut LinenBin>,
    sheets: Query<'w, 's, &'static mut Bedsheet>,
    items: Query<'w, 's, &'static Item>,
    looks: Query<'w, 's, &'static Appearance>,
}

impl LinenWork<'_, '_> {
    pub fn is_bin(&self, target: Entity) -> bool {
        self.bins.contains(target)
    }

    pub fn is_sheet(&self, item: Entity) -> bool {
        self.sheets.contains(item)
    }

    fn noun(&self, entity: Entity) -> String {
        self.looks
            .get(entity)
            .map(|a| a.sprite.clone())
            .unwrap_or_default()
    }

    /// Luna `bedsheetbin/attack_hand` and `attackby`. A sheet goes back in; something
    /// smaller than bulky hides among the sheets, if there are any and nothing is hidden
    /// yet; an empty hand takes a sheet out, and whatever was hidden falls out with it.
    /// `None` when the click isn't one of those. The caller checks reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        hands: &mut Hands,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item();
        let item_noun = active.map(|i| self.noun(i));
        let small = active
            .and_then(|i| self.items.get(i).ok())
            .is_some_and(|i| i.size < 4);
        let is_sheet = active.is_some_and(|i| self.sheets.contains(i));
        let mut bin = self.bins.get_mut(target).ok()?;
        if let Some(item) = active {
            if is_sheet {
                *hands.slot_mut(hands.active) = None;
                commands.entity(item).remove::<HeldBy>().insert(Filed(target));
                bin.returned.push(item);
                bin.amount += 1;
                return Some(told("linen-bin-put", item_noun));
            }
            if bin.amount > 0 && bin.hidden.is_none() && small {
                *hands.slot_mut(hands.active) = None;
                commands.entity(item).remove::<HeldBy>().insert(Filed(target));
                bin.hidden = Some(item);
                return Some(told("linen-bin-hide", item_noun));
            }
            return None;
        }
        if bin.amount == 0 {
            return Some(ClickOutcome::default());
        }
        bin.amount -= 1;
        let hand = hands.active;
        let hidden = bin.hidden.take();
        let mut outcome = ClickOutcome::default();
        match bin.returned.pop() {
            Some(sheet) => {
                commands.entity(sheet).remove::<Filed>().insert(HeldBy(mob));
                *hands.slot_mut(hand) = Some(sheet);
                outcome.to_mob = Some(LocMsg::new("linen-bin-take").arg("item", self.noun(sheet)));
            }
            None => {
                commands.queue(move |world: &mut World| {
                    let Ok(sheet) = crate::objects::spawn_object(world, &ObjectSpec::new(BEDSHEET), mob_pos)
                    else {
                        return;
                    };
                    world.entity_mut(sheet).remove::<GridPos>().insert(HeldBy(mob));
                    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                        *hands.slot_mut(hand) = Some(sheet);
                    }
                });
                outcome.to_mob = Some(LocMsg::new("linen-bin-take").arg("item", BEDSHEET));
            }
        }
        if let Some(hidden) = hidden {
            commands.entity(hidden).remove::<Filed>().insert(GridPos(mob_pos));
            // Only the taker is told (Luna `user <<`).
            outcome.seen = Some(LocMsg::new("linen-bin-falls-out").arg("item", self.noun(hidden)));
        }
        Some(outcome)
    }

    /// Luna `bedsheet/attack_self`: the sheet is dropped, and spread over (or pulled off)
    /// what lies under it.
    pub fn use_self(
        &mut self,
        mob_pos: TilePos,
        hands: &mut Hands,
        item: Entity,
        commands: &mut Commands,
    ) -> bool {
        let Ok(mut sheet) = self.sheets.get_mut(item) else {
            return false;
        };
        sheet.spread = !sheet.spread;
        *hands.slot_mut(hands.active) = None;
        commands.entity(item).remove::<HeldBy>().insert(GridPos(mob_pos));
        true
    }

    /// Luna `bedsheetbin/examine`: how many sheets are left.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let bin = self.bins.get(target).ok()?;
        Some(LocMsg::new("examine-linen-bin").num("count", i64::from(bin.amount)))
    }
}

fn told(id: &str, item: Option<String>) -> ClickOutcome {
    ClickOutcome {
        to_mob: Some(LocMsg::new(id).arg("item", item.unwrap_or_default())),
        seen: None,
    }
}

/// Sprites: a bin full, half full or empty (Luna `update_icon`); a sheet spread or folded.
pub fn update_linen_looks(
    mut bins: Query<(&LinenBin, &mut Appearance), (Changed<LinenBin>, Without<Bedsheet>)>,
    mut sheets: Query<(&Bedsheet, &mut Appearance), Changed<Bedsheet>>,
) {
    for (bin, mut look) in &mut bins {
        let state = match bin.amount {
            0 => "linenbin-empty",
            1..=5 => "linenbin-half",
            _ => "linenbin-full",
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (sheet, mut look) in &mut sheets {
        let state = if sheet.spread { SPREAD } else { "" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

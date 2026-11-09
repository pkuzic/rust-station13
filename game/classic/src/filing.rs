//! Filing cabinets and chest drawers: sheets of paper go in, and come back out by name from
//! their window. Spec: `docs/reference/filing-cabinets.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, NetId, PickListView, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::hands::{Hands, HeldBy};
use crate::paperwork::{Filed, Paper};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const FILING_CABINET: &str = "filing-cabinet";
pub const CHEST_DRAWER: &str = "chest-drawer";
/// How long a drawer shows open after something goes in or comes out, ds (Luna `sleep(5)`).
pub const OPEN_DS: f32 = 5.0;

pub fn is_cabinet(sprite: &str) -> bool {
    matches!(sprite, FILING_CABINET | CHEST_DRAWER)
}

/// A filing cabinet: the sheets in it, the last put in listed first; whether it is bolted
/// down; and until when a drawer shows open.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct FilingCabinet {
    pub contents: Vec<Entity>,
    pub anchored: bool,
    pub open_until: Option<f32>,
    /// Luna `initialize`: sheets on its tile at round start are filed once.
    pub stocked: bool,
}

/// What the mob's open list window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PickListOut(pub Option<PickListView>);

/// Lines picked in an open list window, waiting for the next tick: (the cabinet, line).
#[derive(Component, Debug, Default)]
pub struct PendingPicks(pub Vec<(Entity, usize)>);

pub fn spawn_cabinet(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let (sprite, state) = if proto == CHEST_DRAWER {
        (CHEST_DRAWER, "chestdrawer")
    } else {
        (FILING_CABINET, "tallcabinet")
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(sprite, state),
            FilingCabinet {
                anchored: true,
                ..Default::default()
            },
            rs13_world::Dense,
        ))
        .id()
}

/// Everything clicks on filing cabinets need.
#[derive(SystemParam)]
pub struct FilingWork<'w, 's> {
    cabinets: Query<'w, 's, &'static mut FilingCabinet>,
    papers: Query<'w, 's, &'static Paper>,
    looks: Query<'w, 's, &'static Appearance>,
    time: Res<'w, WorldTime>,
}

impl FilingWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.cabinets.contains(target)
    }

    fn noun(&self, entity: Entity) -> String {
        self.looks
            .get(entity)
            .map_or_else(|_| FILING_CABINET.to_owned(), |a| a.sprite.clone())
    }

    /// Luna `filingcabinet/attackby` and `attack_hand`: a sheet goes in; a wrench bolts it
    /// down or frees it (`wrenched` says it did); anything else won't go in; an empty hand
    /// opens its window, or hears it is empty. The caller checks reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        target: Entity,
        wrench: bool,
        commands: &mut Commands,
    ) -> Option<(ClickOutcome, bool)> {
        let now = self.time.deciseconds();
        let noun = self.noun(target);
        let active = hands.active_item();
        let item_noun = active.map(|i| self.noun(i));
        let mut cabinet = self.cabinets.get_mut(target).ok()?;
        let Some(item) = active else {
            if cabinet.contents.is_empty() {
                return Some((told("cabinet-empty", &noun, None), false));
            }
            commands.entity(mob).insert(PanelViewer(target));
            return Some((ClickOutcome::default(), false));
        };
        if self.papers.contains(item) {
            *hands.slot_mut(hands.active) = None;
            commands.entity(item).remove::<HeldBy>().insert(Filed(target));
            cabinet.contents.push(item);
            cabinet.open_until = Some(now + OPEN_DS);
            return Some((told("cabinet-put", &noun, item_noun.as_deref()), false));
        }
        if wrench {
            cabinet.anchored = !cabinet.anchored;
            let id = if cabinet.anchored {
                "cabinet-wrenched"
            } else {
                "cabinet-unwrenched"
            };
            return Some((told(id, &noun, None), true));
        }
        Some((told("cabinet-wont-fit", &noun, item_noun.as_deref()), false))
    }

    /// Whether it is bolted down, for pulling.
    pub fn anchored(&self, target: Entity) -> Option<bool> {
        self.cabinets.get(target).ok().map(|c| c.anchored)
    }
}

fn told(id: &str, cabinet: &str, item: Option<&str>) -> ClickOutcome {
    let mut msg = LocMsg::new(id).arg("cabinet", cabinet.to_owned());
    if let Some(item) = item {
        msg = msg.arg("item", item.to_owned());
    }
    ClickOutcome {
        to_mob: Some(msg),
        seen: None,
    }
}

/// Luna `filingcabinet/initialize`: the sheets lying on a cabinet's tile go in.
pub fn stock_cabinets(
    mut cabinets: Query<(Entity, &GridPos, &mut FilingCabinet)>,
    papers: Query<(Entity, &GridPos), With<Paper>>,
    mut commands: Commands,
) {
    for (entity, at, mut cabinet) in &mut cabinets {
        if cabinet.stocked {
            continue;
        }
        cabinet.stocked = true;
        for (sheet, pos) in &papers {
            if pos.0 == at.0 {
                commands.entity(sheet).remove::<GridPos>().insert(Filed(entity));
                cabinet.contents.push(sheet);
            }
        }
    }
}

/// Luna `filingcabinet/Topic` (`retrieve`): the picked sheet comes out into the hands, if
/// the cabinet is in reach; the window closes.
#[allow(clippy::type_complexity)]
pub fn process_picks(
    time: Res<WorldTime>,
    mut mobs: Query<(Entity, &GridPos, &mut Hands, &mut PendingPicks)>,
    mut cabinets: Query<(&GridPos, &mut FilingCabinet)>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (mob, mob_pos, mut hands, mut picks) in &mut mobs {
        for (target, line) in std::mem::take(&mut picks.0) {
            let Ok((at, mut cabinet)) = cabinets.get_mut(target) else {
                // Someone else's list (`wizard_items`): left for it.
                picks.0.push((target, line));
                continue;
            };
            commands.entity(mob).remove::<PanelViewer>();
            if !in_reach(mob_pos.0, at.0) || line >= cabinet.contents.len() {
                continue;
            }
            // The window lists the last put in first.
            let index = cabinet.contents.len() - 1 - line;
            let sheet = cabinet.contents.remove(index);
            cabinet.open_until = Some(now + OPEN_DS);
            let hand = if hands.active_item().is_none() {
                Some(hands.active)
            } else if hands.get(hands.active.other()).is_none() {
                Some(hands.active.other())
            } else {
                None
            };
            let mut sheet_cmd = commands.entity(sheet);
            sheet_cmd.remove::<Filed>();
            match hand {
                Some(hand) => {
                    *hands.slot_mut(hand) = Some(sheet);
                    sheet_cmd.insert(HeldBy(mob));
                }
                // Both hands full: it lands at their feet, as Luna's `put_in_hands`.
                None => {
                    sheet_cmd.insert(GridPos(mob_pos.0));
                }
            }
        }
    }
}

/// Keep each viewer's cabinet window in step; close it once the cabinet is out of reach or
/// empty.
pub fn update_cabinet_views(
    mut viewers: Query<(Entity, &GridPos, Option<&PanelViewer>, &mut PickListOut)>,
    cabinets: Query<(&GridPos, &FilingCabinet, &NetId, &Appearance)>,
    papers: Query<&Paper>,
    looks: Query<&Appearance>,
    mut commands: Commands,
) {
    for (mob, mob_pos, viewer, mut out) in &mut viewers {
        if viewer.and_then(|v| looks.get(v.0).ok()).is_some_and(|a| {
            crate::wizard_items::keeps_own_list(&a.sprite)
                || crate::teleporters::keeps_own_list(&a.sprite)
                || crate::devices::keeps_own_list(&a.sprite)
        }) {
            continue;
        }
        let target = viewer.map(|v| v.0).filter(|t| cabinets.contains(*t));
        let shown = target.and_then(|cabinet| {
            let (at, cabinet, id, look) = cabinets.get(cabinet).ok()?;
            if !in_reach(mob_pos.0, at.0) || cabinet.contents.is_empty() {
                return None;
            }
            Some(PickListView {
                target: *id,
                title: look.sprite.clone(),
                rows: cabinet
                    .contents
                    .iter()
                    .rev()
                    .filter_map(|p| papers.get(*p).ok())
                    .map(|p| p.name.clone())
                    .collect(),
            })
        });
        if shown.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != shown {
            out.0 = shown;
        }
    }
}

/// A drawer shows open for a moment after use (Luna `[icon_state]-open`).
pub fn update_cabinet_looks(
    time: Res<WorldTime>,
    mut cabinets: Query<(&mut FilingCabinet, &mut Appearance)>,
) {
    let now = time.deciseconds();
    for (mut cabinet, mut look) in &mut cabinets {
        if cabinet.open_until.is_some_and(|t| now >= t) {
            cabinet.open_until = None;
        }
        let base = if look.sprite == CHEST_DRAWER {
            "chestdrawer"
        } else {
            "tallcabinet"
        };
        let state = if cabinet.open_until.is_some() {
            format!("{base}-open")
        } else {
            base.to_owned()
        };
        if look.state != state {
            look.state = state;
        }
    }
}

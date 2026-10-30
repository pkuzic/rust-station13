//! Tables and racks: things to put items on, and to take apart with a wrench.
//! Spec: `docs/reference/tables.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, TilePos};

use crate::actions::{ClickOutcome, Outbox};
use crate::hands::{Hands, HeldBy};
use crate::time::WorldTime;

/// How long taking a table apart or welding a reinforced one takes (Luna `sleep(50)`).
pub const WORK_DS: f32 = 50.0;

/// A table. Reinforced ones must be weakened with a welding tool before a wrench takes
/// them apart (Luna `status`: 2 strong, 1 weakened).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    pub kind: TableKind,
    pub weakened: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Plain,
    Reinforced,
    Wood,
}

impl TableKind {
    /// What a wrench leaves of it.
    fn parts(self) -> &'static str {
        match self {
            TableKind::Plain => "table-parts",
            TableKind::Reinforced => "table-parts-reinforced",
            TableKind::Wood => "table-parts-wood",
        }
    }
}

/// A rack.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rack;

/// A mob busy with a table: the work is done at `until` if it is still standing where it
/// was and still holds the tool (Luna's `sleep` and checks).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TableWork {
    pub table: Entity,
    pub job: Job,
    pub until: f32,
    pub at: TilePos,
    pub tool: Entity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    TakeApart,
    Weaken,
    Strengthen,
}

/// Prototype ids of tables and racks.
pub fn is_furniture(proto: &str) -> bool {
    matches!(proto, "table" | "table-reinforced" | "table-wood" | "rack")
}

/// Spawn a table or rack: dense, with the look the map gave it.
pub fn spawn_furniture(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: rs13_world::NetId) -> Entity {
    let proto = spec.id.as_str();
    let mut look = spec.state.clone().unwrap_or_default();
    if let Some(dir) = spec.dir {
        look = format!("{look}-{}", dir_name(dir));
    }
    let mut entity = world.spawn((GridPos(pos), id, Appearance::new(proto, look), rs13_world::Dense));
    let kind = match proto {
        "table" => Some(TableKind::Plain),
        "table-reinforced" => Some(TableKind::Reinforced),
        "table-wood" => Some(TableKind::Wood),
        _ => None,
    };
    match kind {
        Some(kind) => entity.insert(Table {
            kind,
            weakened: false,
        }),
        None => entity.insert(Rack),
    };
    entity.id()
}

/// Direction names as they appear in sprite states (`tabledir-southwest`).
fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::NorthEast => "northeast",
        Dir::East => "east",
        Dir::SouthEast => "southeast",
        Dir::South => "south",
        Dir::SouthWest => "southwest",
        Dir::West => "west",
        Dir::NorthWest => "northwest",
    }
}

/// Everything clicking tables, racks and their parts needs.
#[derive(SystemParam)]
pub struct FurnitureWork<'w, 's> {
    tables: Query<'w, 's, (&'static GridPos, &'static mut Table)>,
    racks: Query<'w, 's, &'static GridPos, (With<Rack>, Without<Table>)>,
    looks: Query<'w, 's, &'static Appearance>,
    time: Res<'w, WorldTime>,
}

impl FurnitureWork<'_, '_> {
    fn sprite(&self, entity: Entity) -> Option<&str> {
        self.looks.get(entity).ok().map(|a| a.sprite.as_str())
    }

    /// Whether using `item` in the hand does something here (rack parts become a rack).
    pub fn uses_self(&self, item: Entity) -> bool {
        self.sprite(item) == Some("rack-parts")
    }

    /// Rack parts used in hand: a rack on the mob's tile (Luna `rack_parts/attack_self`).
    pub fn use_self(&mut self, mob_pos: TilePos, hands: &mut Hands, item: Entity, commands: &mut Commands) {
        if !self.uses_self(item) {
            return;
        }
        let active = hands.active;
        *hands.slot_mut(active) = None;
        commands.entity(item).despawn();
        commands.queue(move |world: &mut World| {
            let _ = crate::objects::spawn_object(world, &ObjectSpec::new("rack"), mob_pos);
        });
    }

    /// Clicking a table or rack with an item, or table or rack parts with a wrench. `None`
    /// when these rules don't apply. The caller checks reach; `at` is where the target is
    /// (the mob's own tile for something it holds).
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        hands: &mut Hands,
        target: Entity,
        at: TilePos,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item()?;
        let tool = self.sprite(active).map(str::to_owned);
        let wrench = tool.as_deref() == Some("wrench");

        // Parts and a wrench: a metal sheet (Luna `table_parts/attackby`).
        if wrench && matches!(self.sprite(target), Some("table-parts" | "rack-parts")) {
            for hand in [hands.active, hands.active.other()] {
                if hands.get(hand) == Some(target) {
                    *hands.slot_mut(hand) = None;
                }
            }
            commands.entity(target).despawn();
            commands.queue(move |world: &mut World| {
                let _ = crate::objects::spawn_object(world, &ObjectSpec::new("metal-sheet"), at);
            });
            return Some(ClickOutcome::default());
        }

        if let Ok(pos) = self.racks.get(target) {
            let pos = pos.0;
            if wrench {
                commands.entity(target).despawn();
                commands.queue(move |world: &mut World| {
                    let _ = crate::objects::spawn_object(world, &ObjectSpec::new("rack-parts"), pos);
                });
            } else {
                put_down(hands, active, pos, commands);
            }
            return Some(ClickOutcome::default());
        }

        let (pos, table) = self.tables.get(target).ok()?;
        let (pos, table) = (pos.0, *table);
        let start = |job: Job, commands: &mut Commands, until: f32| {
            commands.entity(mob).insert(TableWork {
                table: target,
                job,
                until,
                at: mob_pos,
                tool: active,
            });
        };
        let until = self.time.deciseconds() + WORK_DS;
        let welder = tool.as_deref() == Some(crate::welding::WELDER);
        Some(match (table.kind, wrench, welder) {
            (TableKind::Reinforced, _, true) => {
                let (job, id) = if table.weakened {
                    (Job::Strengthen, "table-strengthening")
                } else {
                    (Job::Weaken, "table-weakening")
                };
                start(job, commands, until);
                ClickOutcome::told(id)
            }
            (TableKind::Reinforced, true, _) if table.weakened => {
                start(Job::TakeApart, commands, until);
                ClickOutcome::told("table-disassembling-reinforced")
            }
            (TableKind::Plain | TableKind::Wood, true, _) => {
                start(Job::TakeApart, commands, until);
                ClickOutcome::told("table-disassembling")
            }
            // Anything else is put down on the table (Luna `drop_item`).
            _ => {
                put_down(hands, active, pos, commands);
                ClickOutcome::default()
            }
        })
    }
}

/// The item in the active hand goes onto `pos`.
fn put_down(hands: &mut Hands, item: Entity, pos: TilePos, commands: &mut Commands) {
    let active = hands.active;
    *hands.slot_mut(active) = None;
    commands.entity(item).remove::<HeldBy>().insert(GridPos(pos));
}

/// Finish table work that is due, if the mob stayed put and kept its tool.
pub fn finish_table_work(
    time: Res<WorldTime>,
    mut mobs: Query<(Entity, &TableWork, &GridPos, &Hands, &mut Outbox)>,
    mut tables: Query<(&GridPos, &mut Table)>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (mob, work, pos, hands, mut outbox) in &mut mobs {
        if now < work.until {
            continue;
        }
        commands.entity(mob).remove::<TableWork>();
        if pos.0 != work.at || hands.active_item() != Some(work.tool) {
            continue;
        }
        let Ok((table_pos, mut table)) = tables.get_mut(work.table) else {
            continue;
        };
        match work.job {
            Job::Weaken => {
                table.weakened = true;
                outbox.0.push(rs13_i18n::LocMsg::new("table-weakened"));
            }
            Job::Strengthen => {
                table.weakened = false;
                outbox.0.push(rs13_i18n::LocMsg::new("table-strengthened"));
            }
            Job::TakeApart => {
                let at = table_pos.0;
                let parts = table.kind.parts();
                commands.entity(work.table).despawn();
                commands.queue(move |world: &mut World| {
                    let _ = crate::objects::spawn_object(world, &ObjectSpec::new(parts), at);
                });
            }
        }
    }
}

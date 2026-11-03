//! The autolathe (Luna's `autolathe`): feed it sheets, and it makes tools and parts out of them.
//! Spec: `docs/reference/autolathe.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, AutolathePanelView, AutolatheRow, Dense, GridPos, MobStat, NetId, ObjectSpec, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::hands::{Hands, Stack};
use crate::health::Vitals;
use crate::objects::spawn_object;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const AUTOLATHE: &str = "autolathe";
/// What it can hold (Luna `max_m_amount`, `max_g_amount`).
pub const MAX_METAL: i32 = 150_000;
pub const MAX_GLASS: i32 = 75_000;
/// How long making something takes, ds (Luna `spawn(16)`).
pub const MAKE_DS: f32 = 16.0;

/// What it makes, and the metal and glass each takes (Luna `autolathe_recipes`, `m_amt`, `g_amt`).
pub const RECIPES: [(&str, i32, i32); 26] = [
    ("screwdriver", 50, 20),
    ("bucket", 200, 0),
    ("crowbar", 50, 0),
    ("flashlight", 50, 20),
    ("fire-extinguisher", 90, 0),
    ("multitool", 50, 20),
    ("t-ray-scanner", 150, 0),
    ("welder", 30, 30),
    ("wirecutters", 80, 0),
    ("wrench", 150, 0),
    ("welding-helmet", 3000, 1000),
    ("console-screen", 0, 200),
    ("metal-sheet", 3750, 0),
    ("plasteel-sheet", 7500, 0),
    ("glass-sheet", 0, 3750),
    ("rglass-sheet", 1875, 3750),
    ("retractor", 10_000, 5000),
    ("cautery", 5000, 2500),
    ("hemostat", 5000, 2500),
    ("beaker", 0, 500),
    ("beaker-large", 0, 5000),
    ("speed-loader-38", 50_000, 0),
    ("igniter-device", 100, 0),
    ("timer", 100, 0),
    ("scalpel", 10_000, 5000),
    ("circular-saw", 20_000, 10_000),
];

/// What a sheet of this gives it, metal and glass (Luna `m_amt`, `g_amt` of the sheets).
fn sheet(proto: &str) -> Option<(i32, i32)> {
    Some(match proto {
        "metal-sheet" => (3750, 0),
        "plasteel-sheet" => (7500, 0),
        "glass-sheet" => (0, 3750),
        "rglass-sheet" => (1875, 3750),
        _ => return None,
    })
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct Autolathe {
    pub metal: i32,
    pub glass: i32,
    /// What it's making and when that's done, ds.
    pub making: Option<(usize, f32)>,
}

/// Lines of its list a mob pressed this tick.
#[derive(Component, Debug, Default)]
pub struct PendingLatheOps(pub Vec<(Entity, usize)>);

/// What a mob's open autolathe window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct LathePanelOut(pub Option<AutolathePanelView>);

pub fn spawn_autolathe(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(AUTOLATHE, "autolathe"),
            Dense,
            Autolathe::default(),
            PowerUser::new(Channel::Equipment, 10.0),
        ))
        .id()
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// A click: sheets in hand go in, as many as fit; an empty hand opens the window (Luna `attackby`,
/// `attack_hand`).
pub fn click(world: &mut World, mob: Entity, machine: Entity) {
    let Some(item) = world.get::<Hands>(mob).and_then(Hands::active_item) else {
        world.entity_mut(mob).insert(PanelViewer(machine));
        return;
    };
    let proto = world
        .get::<Appearance>(item)
        .map(|a| a.sprite.clone())
        .unwrap_or_default();
    let Some((metal, glass)) = sheet(&proto) else {
        tell(world, mob, LocMsg::new("lathe-refuses"));
        return;
    };
    let Some(state) = world.get::<Autolathe>(machine).copied() else {
        return;
    };
    let held = world.get::<Stack>(item).map_or(1, |s| s.amount as i32);
    let room = |max: i32, have: i32, each: i32| if each > 0 { (max - have) / each } else { i32::MAX };
    let taken = held
        .min(room(MAX_METAL, state.metal, metal))
        .min(room(MAX_GLASS, state.glass, glass));
    if taken <= 0 {
        tell(world, mob, LocMsg::new("lathe-full"));
        return;
    }
    if let Some(mut lathe) = world.get_mut::<Autolathe>(machine) {
        lathe.metal += metal * taken;
        lathe.glass += glass * taken;
    }
    if taken >= held {
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
        world.entity_mut(item).despawn();
    } else if let Some(mut stack) = world.get_mut::<Stack>(item) {
        stack.amount -= taken as u32;
    }
    tell(
        world,
        mob,
        LocMsg::new("lathe-fed").num("value", i64::from(taken)),
    );
}

/// Buttons pressed, and what's done (Luna `Topic`).
pub fn run_autolathes(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut askers = world.query::<(&GridPos, &mut PendingLatheOps)>();
    let asked: Vec<(TilePos, Vec<(Entity, usize)>)> = askers
        .iter_mut(world)
        .filter(|(_, ops)| !ops.0.is_empty())
        .map(|(at, mut ops)| (at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (at, ops) in asked {
        for (machine, line) in ops {
            let near = world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0));
            let powered = world.get::<PowerUser>(machine).is_some_and(|p| p.powered);
            let Some(mut lathe) = world.get_mut::<Autolathe>(machine) else {
                continue;
            };
            let Some((_, metal, glass)) = RECIPES.get(line) else {
                continue;
            };
            if near && powered && lathe.making.is_none() && lathe.metal >= *metal && lathe.glass >= *glass {
                lathe.metal -= metal;
                lathe.glass -= glass;
                lathe.making = Some((line, now + MAKE_DS));
                if let Some(mut look) = world.get_mut::<Appearance>(machine) {
                    look.state = "autolathe_n".to_owned();
                }
            }
        }
    }
    let mut machines = world.query::<(Entity, &GridPos, &Autolathe)>();
    let done: Vec<(Entity, TilePos, usize)> = machines
        .iter(world)
        .filter_map(|(e, at, lathe)| {
            let (line, when) = lathe.making?;
            (now >= when).then_some((e, at.0, line))
        })
        .collect();
    for (machine, at, line) in done {
        if let Some(mut lathe) = world.get_mut::<Autolathe>(machine) {
            lathe.making = None;
        }
        if let Some(mut look) = world.get_mut::<Appearance>(machine) {
            look.state = AUTOLATHE.to_owned();
        }
        // Every recipe is a prototype (`tests/autolathe.rs` checks).
        let _ = spawn_object(world, &ObjectSpec::new(RECIPES[line].0), at);
    }
}

/// Show each player the autolathe they have open.
#[allow(clippy::type_complexity)]
pub fn update_lathe_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut LathePanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &Autolathe, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, lathe, id)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| AutolathePanelView {
                target: *id,
                metal: lathe.metal,
                glass: lathe.glass,
                busy: lathe.making.is_some(),
                rows: RECIPES
                    .iter()
                    .map(|(item, metal, glass)| AutolatheRow {
                        item: (*item).to_owned(),
                        metal: *metal,
                        glass: *glass,
                        can_make: lathe.metal >= *metal && lathe.glass >= *glass,
                    })
                    .collect(),
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

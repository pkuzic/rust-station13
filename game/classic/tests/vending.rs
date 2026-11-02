//! Vending machines. Spec: docs/reference/vending.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_classic::time::WorldTime;
use rs13_classic::vending::{KINDS, PendingVendOps, Vending, VendingPanelOut};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A room with a snack machine against the north wall.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'v': (tile: (id: "floor"), objects: ["vending-snack"]),
    },
    rows: [
        "WWWWW",
        "W.v.W",
        "W...W",
        "WWWWW",
    ],
)"#;

const MACHINE_AT: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let machine = app
        .world_mut()
        .query_filtered::<Entity, With<Vending>>()
        .single(app.world())
        .unwrap();
    // No APC in this room: power it by hand.
    app.world_mut().get_mut::<PowerUser>(machine).unwrap().powered = true;
    (app, machine)
}

fn seconds(app: &mut App, s: f32) {
    for _ in 0..(s * 20.0) as u32 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn on_tile(app: &mut App, at: TilePos) -> Vec<String> {
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    q.iter(app.world())
        .filter(|(_, p)| p.0 == at)
        .map(|(a, _)| a.sprite.clone())
        .collect()
}

#[test]
fn every_product_is_a_prototype() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    for kind in &KINDS {
        for (item, _) in kind.products {
            assert!(
                spawn_object(app.world_mut(), &ObjectSpec::new(*item), TilePos::new(0, 0, 0)).is_ok(),
                "{} sells {item}, which isn't a prototype",
                kind.id
            );
        }
    }
}

#[test]
fn the_window_lists_the_stock_and_vend_drops_one_out_after_a_moment() {
    let (mut app, machine) = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(machine));
    seconds(&mut app, 0.1);
    let view = app
        .world()
        .get::<VendingPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert_eq!(view.machine, "vending-snack");
    assert_eq!(view.products[2].item, "chips");
    assert_eq!(view.products[2].amount, 6);

    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, 2));
    seconds(&mut app, 0.1);
    let view = app
        .world()
        .get::<VendingPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .unwrap();
    assert_eq!(view.products[2].amount, 5);
    assert!(!view.ready, "busy handing it out");
    assert!(
        !on_tile(&mut app, MACHINE_AT).contains(&"chips".to_owned()),
        "not yet"
    );
    seconds(&mut app, 1.0);
    assert!(
        on_tile(&mut app, MACHINE_AT).contains(&"chips".to_owned()),
        "dropped out"
    );
}

#[test]
fn it_runs_out_and_needs_power() {
    let (mut app, machine) = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    for _ in 0..8 {
        app.world_mut()
            .get_mut::<PendingVendOps>(mob)
            .unwrap()
            .0
            .push((machine, 0));
        seconds(&mut app, 1.2);
    }
    let candy = on_tile(&mut app, MACHINE_AT)
        .iter()
        .filter(|s| *s == "candy")
        .count();
    assert_eq!(candy, 6, "six, then sold out");
    assert_eq!(app.world().get::<Vending>(machine).unwrap().stock[0], 0);

    // Cut its power just after a power cycle (every machine cycle), so it stays cut while
    // we look.
    while app.world().resource::<WorldTime>().tick.0 % MACHINE_TICKS != 1 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().get_mut::<PowerUser>(machine).unwrap().powered = false;
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<Appearance>(machine).unwrap().state, "snack-off");
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(machine));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(
        app.world().get::<VendingPanelOut>(mob).unwrap().0.is_none(),
        "dead without power"
    );
}

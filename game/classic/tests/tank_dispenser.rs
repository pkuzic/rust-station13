//! Tank storage units. Spec: docs/reference/tank-dispenser.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::dispensers::{DISPENSER_TANKS, TankDispenser};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'D': (tile: (id: "floor"), objects: ["tank-dispenser"]),
    },
    rows: [
        "WWWW",
        "WD.W",
        "WWWW",
    ],
)"#;

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    app.world_mut().run_schedule(FixedUpdate);
    let mut q = app.world_mut().query_filtered::<Entity, With<TankDispenser>>();
    let dispenser = q.single(app.world()).unwrap();
    (app, mob, dispenser)
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Option<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    (outbox.len() > before).then(|| outbox[before].id.clone())
}

fn held(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().right
}

fn tanks(app: &App, dispenser: Entity) -> u32 {
    app.world().get::<TankDispenser>(dispenser).unwrap().tanks
}

fn look(app: &App, dispenser: Entity) -> String {
    app.world().get::<Appearance>(dispenser).unwrap().state.clone()
}

#[test]
fn an_empty_hand_takes_an_oxygen_tank() {
    let (mut app, mob, dispenser) = setup();
    assert_eq!(look(&app, dispenser), "+oxygen-5");
    assert_eq!(
        click(&mut app, mob, dispenser).as_deref(),
        Some("dispenser-taken")
    );
    let tank = held(&app, mob).expect("the tank is in hand");
    assert_eq!(app.world().get::<Appearance>(tank).unwrap().sprite, "oxygen-tank");
    assert_eq!(app.world().get::<HeldBy>(tank), Some(&HeldBy(mob)));
    assert!(app.world().get::<GridPos>(tank).is_none());
    assert_eq!(tanks(&app, dispenser), DISPENSER_TANKS - 1);
    assert_eq!(look(&app, dispenser), "+oxygen-5", "Luna: 9 / 2 + 1");
}

#[test]
fn tanks_go_back_in_and_come_out_again_until_it_is_empty() {
    let (mut app, mob, dispenser) = setup();
    click(&mut app, mob, dispenser);
    let tank = held(&app, mob).unwrap();
    assert_eq!(click(&mut app, mob, dispenser).as_deref(), Some("dispenser-put"));
    assert_eq!(held(&app, mob), None);
    assert_eq!(tanks(&app, dispenser), DISPENSER_TANKS);
    assert_eq!(
        click(&mut app, mob, dispenser).as_deref(),
        Some("dispenser-taken")
    );
    assert_eq!(held(&app, mob), Some(tank), "the tank put in comes out first");

    // A full unit takes no more.
    app.world_mut().get_mut::<TankDispenser>(dispenser).unwrap().tanks = DISPENSER_TANKS;
    assert_eq!(click(&mut app, mob, dispenser).as_deref(), Some("dispenser-full"));
    assert_eq!(held(&app, mob), Some(tank));

    // Empty it.
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    app.world_mut().get_mut::<TankDispenser>(dispenser).unwrap().tanks = 1;
    click(&mut app, mob, dispenser);
    assert_eq!(look(&app, dispenser), "");
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    assert_eq!(
        click(&mut app, mob, dispenser).as_deref(),
        Some("dispenser-empty")
    );
    assert_eq!(held(&app, mob), None);
}

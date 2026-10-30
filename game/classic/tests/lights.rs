//! Light fixtures, tubes and light switches. Spec: docs/reference/lights.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Zone};
use rs13_classic::lights::{Light, LightStatus};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::{Apc, PowerUser};
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

/// A powered room with a tube light on its north wall, a light switch, a spare tube and a
/// wrench.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "plating")),
        'B': (tile: (id: "plating"), objects: ["smes", (id: "cable", ends: [East])]),
        'Q': (tile: (id: "plating"), objects: ["apc", (id: "cable", ends: [West])]),
        'L': (tile: (id: "plating"), objects: [(id: "light", dir: North)]),
        's': (tile: (id: "plating"), objects: ["light-switch"]),
        't': (tile: (id: "plating"), objects: ["light-tube"]),
        'w': (tile: (id: "plating"), objects: ["wrench"]),
    },
    area_legend: { 'r': "room" },
    area_rows: [
        "       ",
        " rrrrr ",
        " rrrrr ",
        " rrrrr ",
        "       ",
    ],
    rows: [
        "WWWWWWW",
        "WBQL.sW",
        "W.t.w.W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const LIGHT: TilePos = TilePos::new(3, 3, 0);
const BELOW_LIGHT: TilePos = TilePos::new(3, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BELOW_LIGHT);
    ticks(&mut app, 5);
    (app, mob)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world())
        .find(|(_, a)| a.sprite == sprite)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no {sprite}"))
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Option<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    (outbox.len() > before).then(|| outbox.last().unwrap().id.clone())
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

fn light(app: &App, entity: Entity) -> Light {
    *app.world().get::<Light>(entity).unwrap()
}

fn look(app: &App, entity: Entity) -> String {
    app.world().get::<Appearance>(entity).unwrap().state.clone()
}

#[test]
fn a_light_is_lit_by_power_and_its_area_switch() {
    let (mut app, mob) = setup();
    let fixture = find(&mut app, "light");
    let switch = find(&mut app, "light-switch");
    assert!(light(&app, fixture).on);
    assert_eq!(look(&app, fixture), "tube1-north");
    assert_eq!(app.world().get::<PowerUser>(fixture).unwrap().watts, 200.0);
    assert_eq!(look(&app, switch), "light1");

    // The switch is out of reach from under the light; walk over.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(4, 2, 0);
    assert_eq!(click(&mut app, mob, switch), None);
    ticks(&mut app, 1);
    assert!(!light(&app, fixture).on);
    assert_eq!(look(&app, fixture), "tube0-north");
    assert_eq!(app.world().get::<PowerUser>(fixture).unwrap().watts, 0.0);
    assert_eq!(look(&app, switch), "light0");

    click(&mut app, mob, switch);
    ticks(&mut app, 1);
    assert!(light(&app, fixture).on);
}

#[test]
fn without_lighting_power_the_lights_go_out() {
    let (mut app, _) = setup();
    let fixture = find(&mut app, "light");
    let switch = find(&mut app, "light-switch");
    let mut apcs = app.world_mut().query::<&mut Apc>();
    for mut apc in apcs.iter_mut(app.world_mut()) {
        apc.operating = false;
    }
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert!(!light(&app, fixture).on);
    assert_eq!(look(&app, switch), "light-p");
}

#[test]
fn a_lit_tube_burns_a_bare_hand_and_an_unlit_one_comes_out() {
    let (mut app, mob) = setup();
    let fixture = find(&mut app, "light");
    assert_eq!(click(&mut app, mob, fixture).as_deref(), Some("light-burn-hand"));
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Body>(mob).unwrap().organ(Zone::RHand).burn, 5.0);
    assert_eq!(light(&app, fixture).status, LightStatus::Ok);

    // Switch the room off, then take the tube out.
    let switch = find(&mut app, "light-switch");
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(4, 2, 0);
    click(&mut app, mob, switch);
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = BELOW_LIGHT;
    ticks(&mut app, 1);
    assert_eq!(click(&mut app, mob, fixture).as_deref(), Some("light-removed"));
    ticks(&mut app, 1);
    assert_eq!(light(&app, fixture).status, LightStatus::Empty);
    assert_eq!(look(&app, fixture), "tube-empty-north");
    let tube = app
        .world()
        .get::<Hands>(mob)
        .unwrap()
        .right
        .expect("tube in hand");
    assert_eq!(app.world().get::<Appearance>(tube).unwrap().sprite, "light-tube");
    assert_eq!(click(&mut app, mob, fixture).as_deref(), Some("light-inserted"));
    assert_eq!(light(&app, fixture).status, LightStatus::Ok);
}

#[test]
fn an_empty_socket_says_so() {
    let (mut app, mob) = setup();
    let fixture = find(&mut app, "light");
    app.world_mut().get_mut::<Light>(fixture).unwrap().status = LightStatus::Empty;
    assert_eq!(click(&mut app, mob, fixture).as_deref(), Some("light-no-fitting"));
}

#[test]
fn a_burnt_out_tube_is_swapped_for_a_new_one() {
    let (mut app, mob) = setup();
    let fixture = find(&mut app, "light");
    app.world_mut().get_mut::<Light>(fixture).unwrap().status = LightStatus::Burned;
    let spare = find(&mut app, "light-tube");
    hold(&mut app, mob, spare);
    assert_eq!(click(&mut app, mob, fixture).as_deref(), Some("light-replaced"));
    ticks(&mut app, 1);
    assert_eq!(light(&app, fixture).status, LightStatus::Ok);
    assert!(light(&app, fixture).on, "the new tube lights up");
    // The old one dropped out under the light.
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    assert!(
        q.iter(app.world())
            .any(|(a, p)| a.sprite == "light-tube" && a.state == "burned" && p.0 == LIGHT)
    );
}

#[test]
fn hitting_a_light_with_a_wrench_smashes_it() {
    let (mut app, mob) = setup();
    let fixture = find(&mut app, "light");
    let wrench = find(&mut app, "wrench");
    hold(&mut app, mob, wrench);
    // A wrench (force 5) smashes it 72 times in 100. While lit, the metal may shock.
    for _ in 0..20 {
        click(&mut app, mob, fixture);
        if light(&app, fixture).status == LightStatus::Broken {
            break;
        }
    }
    let said = &app.world().get::<Outbox>(mob).unwrap().0;
    assert!(said.iter().any(|m| m.id == "light-smashed"), "{said:?}");
    ticks(&mut app, 1);
    assert_eq!(light(&app, fixture).status, LightStatus::Broken);
    assert!(!light(&app, fixture).on);
    assert_eq!(look(&app, fixture), "tube-broken-north");
}

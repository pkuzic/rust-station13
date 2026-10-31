//! Border firedoors. Spec: docs/reference/firedoors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::alarms::AreaSeals;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::firedoors::Firedoor;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::welding::Welder;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, Dir, GridPos, TileMap, TilePos};

/// A 5×2 room cut in two by a pair of firedoors on the east edge of column 3. The west
/// half is area "west", the east half "east".
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'f': (tile: (id: "floor"), objects: [(id: "firedoor", dir: East)]),
        'c': (tile: (id: "floor"), objects: ["crowbar"]),
        'w': (tile: (id: "floor"), objects: ["welder"]),
    },
    area_legend: { 'a': "west", 'b': "east" },
    area_rows: [
        "       ",
        " aaabb ",
        " aaabb ",
        "       ",
    ],
    rows: [
        "WWWWWWW",
        "Wc.f..W",
        "Ww.f..W",
        "WWWWWWW",
    ],
)"#;

const WEST: TilePos = TilePos::new(1, 1, 0);
const EAST: TilePos = TilePos::new(5, 1, 0);
const UPPER_DOOR: TilePos = TilePos::new(3, 2, 0);
const LOWER_DOOR: TilePos = TilePos::new(3, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    seconds(&mut app, 0.25);
    (app, mob)
}

fn seconds(app: &mut App, s: f32) {
    for _ in 0..(s * 20.0).round() as usize {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn doors(app: &mut App) -> [Entity; 2] {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Firedoor)>();
    let found: Vec<(Entity, TilePos)> = q.iter(app.world()).map(|(e, p, _)| (e, p.0)).collect();
    let at = |pos| found.iter().find(|(_, p)| *p == pos).unwrap().0;
    [at(UPPER_DOOR), at(LOWER_DOOR)]
}

fn door(app: &App, entity: Entity) -> Firedoor {
    *app.world().get::<Firedoor>(entity).unwrap()
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn drain_east(app: &mut App) {
    let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
    let all = atmos.0.air_at(EAST).total_moles() * 10.0;
    atmos.0.remove_at(EAST, all);
}

fn pressure(app: &App, pos: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(pos)
}

/// Close both firedoors with the crowbar.
fn close_both(app: &mut App, mob: Entity) -> [Entity; 2] {
    let crowbar = find(app, "crowbar");
    hold(app, mob, crowbar);
    let both = doors(app);
    for d in both {
        click(app, mob, d);
    }
    both
}

#[test]
fn firedoors_start_open_and_a_crowbar_closes_them_to_hold_vacuum() {
    let (mut app, mob) = setup();
    let both = doors(&mut app);
    assert!(both.iter().all(|d| !door(&app, *d).closed));

    close_both(&mut app, mob);
    assert!(
        both.iter().all(|d| door(&app, *d).closed),
        "closing seals at once"
    );
    assert_eq!(
        app.world().get::<Appearance>(both[0]).unwrap().state,
        "doorc1-east"
    );
    drain_east(&mut app);
    seconds(&mut app, 5.0);
    assert!(
        pressure(&app, EAST) < 1.0,
        "east stays empty: {}",
        pressure(&app, EAST)
    );
    assert!((pressure(&app, WEST) - ONE_ATMOSPHERE).abs() < 0.5);
    assert_eq!(
        app.world().get::<Appearance>(both[0]).unwrap().state,
        "door1-east"
    );
}

#[test]
fn a_crowbar_opens_them_again_and_the_air_flows() {
    let (mut app, mob) = setup();
    let both = close_both(&mut app, mob);
    drain_east(&mut app);
    // Let the closing animation (1.5 s) finish first.
    seconds(&mut app, 2.0);
    for d in both {
        click(&mut app, mob, d);
    }
    // Still sealed while it opens (1.5 s).
    assert!(door(&app, both[0]).closed);
    seconds(&mut app, 2.0);
    assert!(both.iter().all(|d| !door(&app, *d).closed));
    seconds(&mut app, 10.0);
    assert!(
        pressure(&app, EAST) > 20.0,
        "air came through: {}",
        pressure(&app, EAST)
    );
}

#[test]
fn an_air_alarm_seal_closes_them_and_lifting_it_opens_them() {
    let (mut app, _) = setup();
    let both = doors(&mut app);
    app.world_mut()
        .resource_mut::<AreaSeals>()
        .0
        .insert("west".to_owned());
    seconds(&mut app, 0.1);
    assert!(both.iter().all(|d| door(&app, *d).closed));

    app.world_mut().resource_mut::<AreaSeals>().0.clear();
    // It finishes closing (0.6 s), then opens (0.6 s).
    seconds(&mut app, 2.0);
    assert!(both.iter().all(|d| !door(&app, *d).closed));
}

#[test]
fn a_welded_firedoor_ignores_the_crowbar_and_the_seal() {
    let (mut app, mob) = setup();
    let both = close_both(&mut app, mob);
    seconds(&mut app, 2.0);
    let welder = find(&mut app, "welder");
    hold(&mut app, mob, welder);
    app.world_mut().get_mut::<Welder>(welder).unwrap().lit = true;
    click(&mut app, mob, both[0]);
    assert!(door(&app, both[0]).blocked);

    let crowbar = find(&mut app, "crowbar");
    hold(&mut app, mob, crowbar);
    click(&mut app, mob, both[0]);
    seconds(&mut app, 2.0);
    assert!(door(&app, both[0]).closed, "welded: the crowbar does nothing");

    // A seal and its end leave it shut; the unwelded one opens again.
    app.world_mut()
        .resource_mut::<AreaSeals>()
        .0
        .insert("west".to_owned());
    seconds(&mut app, 0.5);
    app.world_mut().resource_mut::<AreaSeals>().0.clear();
    seconds(&mut app, 1.0);
    assert!(door(&app, both[0]).closed);
    assert!(!door(&app, both[1]).closed);
}

#[test]
fn nobody_walks_across_a_closed_edge_but_along_it_is_fine() {
    let (mut app, mob) = setup();
    close_both(&mut app, mob);
    let walker = spawn_player_mob(app.world_mut(), TilePos::new(4, 2, 0));
    let walk = |app: &mut App, dir: Dir| {
        app.world_mut().get_mut::<MoveIntent>(walker).unwrap().dir = Some(dir);
        seconds(app, 0.5);
        app.world_mut().get_mut::<MoveIntent>(walker).unwrap().dir = None;
        seconds(app, 0.5);
        app.world().get::<GridPos>(walker).unwrap().0
    };
    assert_eq!(walk(&mut app, Dir::West), TilePos::new(4, 2, 0), "west is sealed");
    // South-west crosses the sealed edge both ways round, so it slides south, along the
    // edge, and no further.
    assert_eq!(walk(&mut app, Dir::SouthWest), TilePos::new(4, 1, 0));
    assert_eq!(
        walk(&mut app, Dir::West),
        TilePos::new(4, 1, 0),
        "west is sealed here too"
    );
    assert_eq!(
        walk(&mut app, Dir::North),
        TilePos::new(4, 2, 0),
        "along the edge"
    );
}

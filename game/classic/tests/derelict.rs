//! The derelict round. Spec: docs/reference/derelict.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::alarms::AirAlarm;
use rs13_classic::derelict::{self, ARRIVAL_AREA, Derelict, SCAVENGER};
use rs13_classic::equipment::Equipment;
use rs13_classic::generators::{Pacman, SUPER_PACMAN, SUPER_PACMAN_OUTPUT, URANIUM_SHEET};
use rs13_classic::hands::{Hands, Stack};
use rs13_classic::jobs::CardOwner;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::Apc;
use rs13_classic::rng::GameRng;
use rs13_classic::storage::{InStorage, Storage};
use rs13_classic::windows::WindowPane;
use rs13_world::{Appearance, GridPos, MapLayer, ObjectSpec, Slot, TileMap, TilePos};

/// A room on the left, its floor kept whole by landmarks on the top row (y 4: rows count up from
/// the bottom); the arrival shuttle on the right.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'n': (tile: (id: "floor"), objects: ["landmark-derelict-nodamage"]),
        'S': (tile: (id: "floor"), objects: ["landmark-derelict-supplycrate", "landmark-derelict-nodamage"]),
        'P': (tile: (id: "floor"), objects: ["landmark-derelict-superpacman", "landmark-derelict-nodamage"]),
        'G': (tile: (id: "floor"), objects: ["landmark-derelict-glass", "landmark-derelict-noblast"]),
        'O': (tile: (id: "floor"), objects: ["landmark-derelict-o2crate", "landmark-derelict-noblast"]),
        'p': (tile: (id: "floor"), objects: [(id: "window-pane", dir: North)]),
        'Q': (tile: (id: "floor"), objects: ["apc"]),
        'A': (tile: (id: "floor"), objects: ["air-alarm"]),
    },
    area_legend: { 'r': "room", 'a': "shuttle-arrival-station" },
    area_rows: [
        "rrrrrrrrrrraaaaa",
        "rrrrrrrrrrraaaaa",
        "rrrrrrrrrrraaaaa",
        "rrrrrrrrrrraaaaa",
        "rrrrrrrrrrraaaaa",
        "rrrrrrrrrrraaaaa",
    ],
    rows: [
        "WWWWWWWWWWWWWWWW",
        "WnnnnSPnnnW....W",
        "WGO..pppppW....W",
        "WQ.A......W....W",
        "W.........W....W",
        "WWWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn tile(app: &mut App, x: i32, y: i32) -> String {
    let world = app.world_mut();
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .next()
        .unwrap()
        .0
        .tile(TilePos::new(x, y, 0))
        .unwrap()
        .id
        .clone()
}

fn things_at(app: &mut App, at: TilePos) -> Vec<String> {
    let world = app.world_mut();
    let mut q = world.query::<(&GridPos, &Appearance)>();
    let mut found: Vec<String> = q
        .iter(world)
        .filter(|(p, _)| p.0 == at)
        .map(|(_, a)| a.sprite.clone())
        .collect();
    found.sort();
    found
}

#[test]
fn landmarks_put_out_supplies_and_keep_their_floor_whole() {
    let mut app = setup();
    derelict::wreck(app.world_mut());
    assert!(app.world().get_resource::<Derelict>().is_some());
    for x in 1..10 {
        assert_eq!(tile(&mut app, x, 4), "floor", "kept whole at {x}");
    }
    let crate_tile = things_at(&mut app, TilePos::new(5, 4, 0));
    for want in [
        "crate-engineering",
        "insulated-gloves",
        "magboots",
        "multitool",
        "toolbox-electrical",
    ] {
        assert!(crate_tile.iter().any(|s| s == want), "{want} in {crate_tile:?}");
    }
    assert_eq!(
        crate_tile
            .iter()
            .filter(|s| *s == "emergency-oxygen-engi")
            .count(),
        4
    );

    let world = app.world_mut();
    let mut generators = world.query::<(&Pacman, &GridPos)>();
    let (generator, at) = generators.single(world).unwrap();
    assert!(generator.super_type);
    assert_eq!(at.0, TilePos::new(6, 4, 0));
    assert_eq!(generator.fuel(), URANIUM_SHEET);
    assert_eq!(generator.output(), SUPER_PACMAN_OUTPUT);
    let mut stacks = world.query::<(&Appearance, &Stack, &GridPos)>();
    let uranium: u32 = stacks
        .iter(world)
        .filter(|(a, _, p)| a.sprite == URANIUM_SHEET && p.0 == TilePos::new(6, 4, 0))
        .map(|(_, s, _)| s.amount)
        .sum();
    assert_eq!(uranium, 100);
    let glass: u32 = stacks
        .iter(world)
        .filter(|(a, _, _)| a.sprite == "glass-sheet")
        .map(|(_, s, _)| s.amount)
        .sum();
    assert!((5..=50).contains(&glass), "{glass}");
}

#[test]
fn the_station_is_left_flat_broken_and_strewn() {
    // Over many tries, every kind of damage turns up, and never on a kept tile.
    let (mut no_cell, mut broken, mut flat, mut plating, mut panes_gone, mut alarms_broken) =
        (0, 0, 0, 0, 0, 0);
    for seed in 1..=20 {
        let mut app = setup();
        app.world_mut().insert_resource(GameRng::seeded(seed));
        derelict::wreck(app.world_mut());
        let world = app.world_mut();
        // Now and then a big blast takes a machine with it, as in Luna.
        let Some(apc) = world.query::<&Apc>().iter(world).next().cloned() else {
            continue;
        };
        match apc.cell {
            None => no_cell += 1,
            Some((charge, _)) => {
                assert_eq!(charge, 0.0);
                flat += 1;
            }
        }
        if apc.broken {
            assert!(!apc.operating);
            broken += 1;
        }
        let panes = world.query::<&WindowPane>().iter(world).count();
        panes_gone += 5 - panes;
        if world
            .query::<&AirAlarm>()
            .iter(world)
            .next()
            .is_some_and(|a| a.broken)
        {
            alarms_broken += 1;
        }
        for x in 1..10 {
            if tile(&mut app, x, 1) == "plating" {
                plating += 1;
            }
        }
    }
    assert!(no_cell > 0 && flat > 0 && broken > 0, "{no_cell} {flat} {broken}");
    assert!(
        plating > 0 && panes_gone > 0 && alarms_broken > 10,
        "{plating} {panes_gone} {alarms_broken}"
    );
}

#[test]
fn a_super_pacman_burns_uranium() {
    let mut app = setup();
    let generator = spawn_object(
        app.world_mut(),
        &ObjectSpec::new(SUPER_PACMAN),
        TilePos::new(8, 4, 0),
    )
    .unwrap();
    let pacman = *app.world().get::<Pacman>(generator).unwrap();
    assert!(pacman.super_type);
    let plain = spawn_object(app.world_mut(), &ObjectSpec::new("pacman"), TilePos::new(9, 4, 0)).unwrap();
    assert!(!app.world().get::<Pacman>(plain).unwrap().super_type);
}

#[test]
fn everyone_comes_aboard_on_the_arrival_shuttle_as_a_scavenger() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let old_hat = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("hardhat-red"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    rs13_classic::jobs::wear(app.world_mut(), mob, old_hat);
    derelict::wreck(app.world_mut());
    derelict::setup_round(app.world_mut());
    let world = app.world_mut();
    let at = world.get::<GridPos>(mob).unwrap().0;
    let area = {
        let mut maps = world.query::<&MapLayer>();
        maps.iter(world).next().unwrap().0.area(at).map(str::to_owned)
    };
    assert_eq!(area.as_deref(), Some(ARRIVAL_AREA));
    assert!(world.get_entity(old_hat).is_err(), "what they wore is gone");

    let equipment = world.get::<Equipment>(mob).unwrap().clone();
    let on = |slot: Slot| {
        equipment
            .get(slot)
            .map(|item| world.get::<Appearance>(item).unwrap().sprite.clone())
    };
    assert_eq!(on(Slot::Uniform).as_deref(), Some("black-jumpsuit"));
    assert_eq!(on(Slot::Suit).as_deref(), Some("firesuit"));
    assert_eq!(on(Slot::Mask).as_deref(), Some("gas-mask"));
    assert_eq!(on(Slot::Head).as_deref(), Some("hardhat-red"));
    assert_eq!(on(Slot::Ears).as_deref(), Some("headset"));
    assert_eq!(on(Slot::Belt).as_deref(), Some("utility-belt"));
    assert!(matches!(
        on(Slot::Shoes).as_deref(),
        Some("magboots" | "black-shoes")
    ));
    let card = equipment.get(Slot::Id).unwrap();
    assert_eq!(world.get::<CardOwner>(card).unwrap().job, SCAVENGER);
    assert_eq!(world.get::<CardOwner>(card).unwrap().owner, Some(mob));

    let bag = equipment.get(Slot::Back).unwrap();
    let packed: Vec<String> = world
        .get::<Storage>(bag)
        .unwrap()
        .items
        .iter()
        .map(|i| world.get::<Appearance>(*i).unwrap().sprite.clone())
        .collect();
    for want in ["circuit-box", "high-power-cell", "emergency-oxygen-engi"] {
        assert!(packed.iter().any(|s| s == want), "{want} in {packed:?}");
    }
    let mut boxes = world.query::<(Entity, &Appearance)>();
    let circuit_box = boxes
        .iter(world)
        .find(|(_, a)| a.sprite == "circuit-box")
        .map(|(e, _)| e)
        .unwrap();
    let mut inside = world.query::<(&InStorage, &Appearance)>();
    assert_eq!(
        inside
            .iter(world)
            .filter(|(s, a)| s.0 == circuit_box && a.sprite == "circuit-board")
            .count(),
        7
    );
    let left = world.get::<Hands>(mob).unwrap().left.unwrap();
    assert!(
        world
            .get::<Appearance>(left)
            .unwrap()
            .sprite
            .starts_with("toolbox-")
    );
}

#[test]
fn someone_joining_late_arrives_the_same_way_only_in_this_round() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    derelict::late_join(app.world_mut(), mob);
    assert_eq!(app.world().get::<GridPos>(mob).unwrap().0, TilePos::new(2, 1, 0));
    derelict::wreck(app.world_mut());
    derelict::late_join(app.world_mut(), mob);
    assert!(app.world().get::<GridPos>(mob).unwrap().0.x > 10);
}

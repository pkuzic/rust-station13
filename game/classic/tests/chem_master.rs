//! The CheMaster. Spec: docs/reference/chemistry.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::chem_master::{ChemMaster, MasterOp, MasterPanelOut, PendingMasterOps};
use rs13_classic::chemistry::Reagents;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'm': (tile: (id: "floor"), objects: ["chem-master"]),
    },
    rows: [
        "WWWWW",
        "Wm..W",
        "W...W",
        "WWWWW",
    ],
)"#;
const NEAR: TilePos = TilePos::new(2, 1, 0);

/// Someone next to a CheMaster with this in a beaker, loaded.
fn setup(contents: &[(&str, f32)]) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    let beaker = spawn_object(app.world_mut(), &ObjectSpec::new("beaker"), NEAR).unwrap();
    for (reagent, amount) in contents {
        app.world_mut()
            .get_mut::<Reagents>(beaker)
            .unwrap()
            .add(reagent, *amount);
    }
    app.world_mut().run_schedule(FixedUpdate);
    let mut q = app.world_mut().query_filtered::<Entity, With<ChemMaster>>();
    let machine = q.single(app.world()).unwrap();
    click(&mut app, mob, beaker);
    click(&mut app, mob, machine);
    assert_eq!(
        app.world().get::<ChemMaster>(machine).unwrap().beaker,
        Some(beaker)
    );
    (app, mob, machine, beaker)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn press(app: &mut App, mob: Entity, machine: Entity, op: MasterOp) {
    app.world_mut()
        .get_mut::<PendingMasterOps>(mob)
        .unwrap()
        .0
        .push((machine, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn made(app: &mut App, sprite: &str) -> Vec<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Appearance, &GridPos)>();
    q.iter(app.world())
        .filter(|(_, a, _)| a.sprite == sprite)
        .map(|(e, ..)| e)
        .collect()
}

#[test]
fn it_moves_a_line_to_the_buffer_and_shows_it() {
    let (mut app, mob, machine, beaker) = setup(&[("anti_toxin", 25.0)]);
    press(&mut app, mob, machine, MasterOp::Add(0));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().get::<Reagents>(beaker).unwrap().get("anti_toxin"),
        15.0
    );
    assert_eq!(
        app.world().get::<Reagents>(machine).unwrap().get("anti_toxin"),
        10.0
    );
    let view = app
        .world()
        .get::<MasterPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert_eq!(view.buffer, vec![("anti_toxin".to_owned(), 10)]);
    assert_eq!(view.beaker, Some(vec![("anti_toxin".to_owned(), 15)]));
    assert_eq!(app.world().get::<Appearance>(machine).unwrap().state, "mixer1");

    press(&mut app, mob, machine, MasterOp::Remove(0));
    assert_eq!(
        app.world().get::<Reagents>(beaker).unwrap().get("anti_toxin"),
        25.0
    );
    assert_eq!(app.world().get::<Reagents>(machine).unwrap().total(), 0.0);
}

#[test]
fn it_makes_a_pill_someone_can_swallow() {
    let (mut app, mob, machine, _) = setup(&[("anti_toxin", 20.0)]);
    press(&mut app, mob, machine, MasterOp::Pill);
    assert!(
        made(&mut app, "pill-made").is_empty(),
        "nothing in the buffer yet"
    );
    press(&mut app, mob, machine, MasterOp::Add(0));
    press(&mut app, mob, machine, MasterOp::Add(0));
    press(&mut app, mob, machine, MasterOp::Pill);
    let pills = made(&mut app, "pill-made");
    assert_eq!(pills.len(), 1);
    assert_eq!(
        app.world().get::<Reagents>(pills[0]).unwrap().get("anti_toxin"),
        20.0
    );
    assert_eq!(app.world().get::<Reagents>(machine).unwrap().total(), 0.0);

    click(&mut app, mob, pills[0]);
    click(&mut app, mob, mob);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(
        app.world().get::<Reagents>(mob).unwrap().get("anti_toxin") > 15.0,
        "swallowed"
    );
    assert!(app.world().get_entity(pills[0]).is_err(), "gone");
}

#[test]
fn a_bottle_only_takes_thirty_units() {
    let (mut app, mob, machine, _) = setup(&[("water", 50.0)]);
    for _ in 0..5 {
        press(&mut app, mob, machine, MasterOp::Add(0));
    }
    press(&mut app, mob, machine, MasterOp::Bottle);
    let bottles = made(&mut app, "bottle-made");
    assert_eq!(bottles.len(), 1);
    assert_eq!(
        app.world().get::<Reagents>(bottles[0]).unwrap().get("water"),
        30.0
    );
    assert!((app.world().get::<Reagents>(machine).unwrap().get("water") - 20.0).abs() < 0.01);
}

#[test]
fn ejecting_clears_the_buffer_and_drops_the_beaker() {
    let (mut app, mob, machine, beaker) = setup(&[("water", 20.0)]);
    press(&mut app, mob, machine, MasterOp::Add(0));
    press(&mut app, mob, machine, MasterOp::Eject);
    assert!(app.world().get::<ChemMaster>(machine).unwrap().beaker.is_none());
    assert_eq!(app.world().get::<Reagents>(machine).unwrap().total(), 0.0);
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().get("water"), 10.0);
    assert!(app.world().get::<GridPos>(beaker).is_some());
    assert_eq!(app.world().get::<Appearance>(machine).unwrap().state, "mixer0");
}

/// The window names whatever is in the beaker, so every reagent needs a name in both languages.
#[test]
fn every_reagent_has_a_name() {
    let made = [
        "anti_toxin",
        "kelotane",
        "tricordrazine",
        "dexalinp",
        "synaptizine",
        "cryoxadone",
        "toxin",
        "nutriment",
        "gold",
        "silver",
        "plasma",
        "inaprovaline",
        "stoxin",
        "dermaline",
        "bicaridine",
        "dexalin",
    ];
    for ftl in [
        include_str!("../locales/en/messages.ftl"),
        include_str!("../locales/ru/messages.ftl"),
    ] {
        for reagent in made
            .iter()
            .chain(rs13_classic::chem_dispenser::DISPENSABLE.iter())
        {
            assert!(ftl.contains(&format!("\nreagent-{reagent} = ")), "{reagent}");
        }
    }
}

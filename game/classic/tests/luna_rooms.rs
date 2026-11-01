//! NSV Luna's toxins lab and incinerator, imported from the original map, burn and vent as
//! Luna's do. Spec: docs/reference/map-import.md, igniters.md, blast-doors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::blastdoors::{BlastDoor, DoorButton};
use rs13_classic::fires::Fire;
use rs13_classic::igniters::{Igniter, IgnitionSwitch};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::{ClassicPlugin, maps};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Dir, GridPos, MapLayer, TileMap, TilePos};

fn load(text: &str) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(text, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    seconds(&mut app, 2);
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn place<C: Component>(app: &mut App, keep: impl Fn(&C) -> bool) -> (Entity, TilePos) {
    let mut q = app.world_mut().query::<(Entity, &C, &GridPos)>();
    q.iter(app.world())
        .find(|(_, c, _)| keep(c))
        .map(|(e, _, p)| (e, p.0))
        .expect("on the map")
}

/// A player standing on a free tile beside `at`.
fn player_beside(app: &mut App, at: TilePos) -> Entity {
    let map = {
        let mut q = app.world_mut().query::<&MapLayer>();
        q.single(app.world()).unwrap().0.clone()
    };
    let free = [
        Dir::North,
        Dir::South,
        Dir::East,
        Dir::West,
        Dir::NorthEast,
        Dir::NorthWest,
    ]
    .into_iter()
    .map(|d| at.step(d))
    .find(|p| map.tile(*p).is_some_and(|t| !t.solid && t.id != "space"))
    .expect("a free tile beside it");
    spawn_player_mob(app.world_mut(), free)
}

fn press(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn fires(app: &mut App) -> usize {
    app.world_mut().query::<&Fire>().iter(app.world()).count()
}

#[test]
fn the_toxins_lab_lights_its_burn_chamber_and_vents_it_to_space() {
    let mut app = load(maps::LUNA_TOXINS);
    let map_spawns = {
        let mut q = app.world_mut().query::<&MapLayer>();
        q.single(app.world()).unwrap().0.spawns.clone()
    };
    assert_eq!(map_spawns.len(), 3, "Luna's Scientist start points");
    let cards = app
        .world_mut()
        .query::<&IdCard>()
        .iter(app.world())
        .filter(|c| c.access.iter().any(|a| a == "tox"))
        .count();
    assert_eq!(cards, 3, "a scientist's card on each");

    let (_, chamber) = place::<Igniter>(&mut app, |i| i.id == "toxins");
    let (switch, switch_at) = place::<IgnitionSwitch>(&mut app, |s| s.id == "toxins");
    let lab = switch_at;
    {
        let atmos = &app.world().resource::<StationAtmos>().0;
        assert_ne!(
            atmos.zone_at(chamber),
            atmos.zone_at(lab),
            "the chamber is sealed off"
        );
        assert!(atmos.zone_at(chamber).is_some(), "and holds air, not space");
    }
    let mut plasma = GasMix::default();
    plasma.temperature = T20C;
    plasma.set(Gas::Plasma, 60.0);
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(chamber, &plasma);

    let scientist = player_beside(&mut app, switch_at);
    press(&mut app, scientist, switch);
    let mut most = 0;
    for _ in 0..30 {
        seconds(&mut app, 1);
        most = most.max(fires(&mut app));
    }
    assert!(most > 0, "the chamber burns");
    {
        let atmos = &app.world().resource::<StationAtmos>().0;
        assert!(
            atmos.air_at(lab).get(Gas::CarbonDioxide) < 0.01,
            "the lab doesn't"
        );
    }

    // The vent control opens the chamber's blast doors to space.
    let (button, button_at) = place::<DoorButton>(&mut app, |b| b.id == "toxinsgasexp");
    let ventor = player_beside(&mut app, button_at);
    let before = app.world().resource::<StationAtmos>().0.pressure_at(chamber);
    press(&mut app, ventor, button);
    seconds(&mut app, 10);
    let doors_open = app
        .world_mut()
        .query::<&BlastDoor>()
        .iter(app.world())
        .filter(|d| d.id == "toxinsgasexp")
        .all(|d| !d.blocks());
    assert!(doors_open);
    let after = app.world().resource::<StationAtmos>().0.pressure_at(chamber);
    assert!(after < before / 4.0, "the chamber vents: {before} -> {after} kPa");
}

#[test]
fn the_incinerator_has_its_igniter_switch_and_shut_vent() {
    let mut app = load(maps::LUNA_INCINERATOR);
    let (_, chamber) = place::<Igniter>(&mut app, |i| i.id == "Incinerator");
    let (_, switch_at) = place::<IgnitionSwitch>(&mut app, |s| s.id == "Incinerator");
    let vent_shut = app
        .world_mut()
        .query::<&BlastDoor>()
        .iter(app.world())
        .filter(|d| d.id == "Incernator Vent")
        .all(BlastDoor::blocks);
    assert!(vent_shut, "the vent starts shut");
    let atmos = &app.world().resource::<StationAtmos>().0;
    assert_ne!(
        atmos.zone_at(chamber),
        atmos.zone_at(switch_at),
        "the chamber is sealed off"
    );
}

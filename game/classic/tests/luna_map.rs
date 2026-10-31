//! NSV Luna's engineering section, imported from the original map.
//! Spec: docs/reference/map-import.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::{Entity, With, Without};
use rs13_classic::access::IdCard;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::doors::Airlock;
use rs13_classic::machines::{PipeNets, Scrubber, Vent};
use rs13_classic::objects::spawn_map;
use rs13_classic::power::{Apc, AreaPower, Channel, Powernets, Smes};
use rs13_classic::{ClassicPlugin, maps};
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{GridPos, MapLayer, TileMap};

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::LUNA_ENGINEERING, 0).unwrap(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    app
}

#[test]
fn the_imported_map_loads_and_runs() {
    let mut app = setup();
    for _ in 0..200 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let map = {
        let mut q = app.world_mut().query::<&MapLayer>();
        q.single(app.world()).unwrap().0.clone()
    };
    assert_eq!((map.width, map.height), (34, 25));
    assert_eq!(map.spawns.len(), 5, "Luna's Engineer start points");

    // An engineer's card waits on every start point.
    let mut cards = app.world_mut().query::<(&GridPos, &IdCard)>();
    for spawn in &map.spawns {
        assert!(
            cards
                .iter(app.world())
                .any(|(p, c)| p.0 == *spawn && c.access.iter().any(|a| a == "engine")),
            "card at {spawn:?}"
        );
    }

    // Doors keep Luna's access.
    let mut doors = app.world_mut().query::<&Airlock>();
    let doors: Vec<Airlock> = doors.iter(app.world()).cloned().collect();
    assert!(doors.len() >= 20, "{} doors", doors.len());
    assert!(
        doors.iter().any(|d| d.req_access == ["construction"]),
        "the CE's office"
    );

    // Every APC's area is powered from the engine's SMES units through Luna's cabling.
    let mut apcs = app.world_mut().query::<&Apc>();
    let areas: Vec<String> = apcs.iter(app.world()).map(|a| a.area.clone()).collect();
    assert!(!areas.is_empty());
    let power = app.world().resource::<AreaPower>();
    for area in &areas {
        assert!(
            power.powered(Some(area), Channel::Equipment),
            "{area} is unpowered"
        );
    }

    // ...on the grid, not just their cells: each APC shares a network with a SMES. The
    // starboard maintenance APC's cable runs north out of the imported region, to a grid
    // we don't import, so it lives on its cell.
    let mut apcs = app.world_mut().query_filtered::<(Entity, &Apc), Without<Smes>>();
    let apcs: Vec<(Entity, String)> = apcs.iter(app.world()).map(|(e, a)| (e, a.area.clone())).collect();
    let mut batteries = app.world_mut().query_filtered::<Entity, With<Smes>>();
    let batteries: Vec<Entity> = batteries.iter(app.world()).collect();
    let nets = app.world().resource::<Powernets>();
    for (apc, area) in apcs.iter().filter(|(_, area)| area != "maint-starboard") {
        let net = nets.net_of.get(apc);
        assert!(
            net.is_some() && batteries.iter().any(|b| nets.net_of.get(b) == net),
            "{area}'s APC is off the grid"
        );
    }

    // Luna's supply and scrubber lines run side by side but stay two networks: every vent
    // on one, every scrubber on the other.
    let mut vents = app
        .world_mut()
        .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<Vent>>();
    let vents: Vec<_> = vents.iter(app.world()).collect();
    let mut scrubbers = app
        .world_mut()
        .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<Scrubber>>();
    let scrubbers: Vec<_> = scrubbers.iter(app.world()).collect();
    let nets = app.world().resource::<PipeNets>();
    let supply = nets.net_of[&vents[0]];
    let waste = nets.net_of[&scrubbers[0]];
    assert_ne!(supply, waste);
    let on_supply = vents
        .iter()
        .filter(|v| nets.net_of.get(v) == Some(&supply))
        .count();
    assert!(
        on_supply >= vents.len() - 1,
        "{on_supply} of {} vents",
        vents.len()
    );
    assert!(scrubbers.iter().all(|s| nets.net_of.get(s) == Some(&waste)));

    // The air is standard everywhere inside.
    let atmos = app.world().resource::<StationAtmos>();
    let mut floors = 0;
    for pos in map.spawns.iter() {
        assert!((atmos.0.pressure_at(*pos) - ONE_ATMOSPHERE).abs() < 1.0);
        floors += 1;
    }
    assert_eq!(floors, 5);
}

#[test]
fn the_engines_generators_feed_the_smes_terminals() {
    // Luna's engine: the generators share a network with the SMES bank's terminals, so their
    // power charges the SMES units, which feed the station (`power.md`, `engine.md`).
    let mut app = setup();
    app.world_mut().run_schedule(FixedUpdate);
    let mut tegs = app
        .world_mut()
        .query_filtered::<Entity, With<rs13_classic::teg::Teg>>();
    let tegs: Vec<Entity> = tegs.iter(app.world()).collect();
    let mut terminals = app
        .world_mut()
        .query_filtered::<Entity, With<rs13_classic::power::Terminal>>();
    let terminals: Vec<Entity> = terminals.iter(app.world()).collect();
    assert_eq!(tegs.len(), 2);
    assert!(terminals.len() >= 9);
    let nets = app.world().resource::<Powernets>();
    for teg in tegs {
        let net = nets.net_of.get(&teg);
        assert!(net.is_some() && terminals.iter().any(|t| nets.net_of.get(t) == net));
    }
}

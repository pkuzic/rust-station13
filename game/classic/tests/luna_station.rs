//! All of NSV Luna's station: four decks, imported from the original map and joined by its
//! ladders and stairs. Spec: docs/reference/map-import.md, multiz.md.

use std::collections::BTreeSet;

use bevy_app::{App, FixedUpdate};
use rs13_classic::access::IdCard;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::multiz::{Ladder, Stairs, other_deck};
use rs13_classic::objects::spawn_station;
use rs13_classic::{ClassicPlugin, maps};
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{GridPos, MapLayer, TileMap, TilePos};

fn station() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let decks: Vec<TileMap> = maps::LUNA_STATION
        .iter()
        .enumerate()
        .map(|(z, text)| TileMap::from_ron(text, z as i32).unwrap())
        .collect();
    let problems = spawn_station(app.world_mut(), decks);
    assert!(problems.is_empty(), "{problems:?}");
    app
}

#[test]
fn the_whole_station_loads_and_runs() {
    let mut app = station();
    for _ in 0..100 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let decks: Vec<TileMap> = app
        .world_mut()
        .query::<&MapLayer>()
        .iter(app.world())
        .map(|m| m.0.clone())
        .collect();
    assert_eq!(decks.len(), 4);
    let spawns: Vec<TilePos> = decks.iter().flat_map(|d| d.spawns.clone()).collect();
    assert!(
        spawns.len() > 60,
        "Luna's start points on every deck: {}",
        spawns.len()
    );
    let cards = app.world_mut().query::<&IdCard>().iter(app.world()).count();
    assert!(cards >= 40, "job cards on the start points: {cards}");
    // Start points have air.
    let atmos = &app.world().resource::<StationAtmos>().0;
    let breathable = spawns
        .iter()
        .filter(|p| (atmos.pressure_at(**p) - ONE_ATMOSPHERE).abs() < 10.0)
        .count();
    assert!(
        breathable * 10 >= spawns.len() * 9,
        "{breathable} of {} start points",
        spawns.len()
    );
}

#[test]
fn every_ladder_and_stair_lands_on_the_next_deck() {
    let mut app = station();
    let ladders: Vec<(TilePos, Ladder)> = app
        .world_mut()
        .query::<(&GridPos, &Ladder)>()
        .iter(app.world())
        .map(|(p, l)| (p.0, *l))
        .collect();
    assert!(ladders.len() >= 20, "{} ladder ends", ladders.len());
    let ends: BTreeSet<TilePos> = ladders.iter().map(|(p, _)| *p).collect();
    for (at, ladder) in &ladders {
        let to = other_deck(*at, ladder.down);
        assert!(
            ends.contains(&to),
            "the ladder at {at:?} leads to a ladder end at {to:?}"
        );
    }
    let stairs: Vec<(TilePos, Stairs)> = app
        .world_mut()
        .query::<(&GridPos, &Stairs)>()
        .iter(app.world())
        .map(|(p, s)| (p.0, *s))
        .collect();
    let entries: BTreeSet<TilePos> = stairs
        .iter()
        .filter(|(_, s)| !s.active)
        .map(|(p, _)| *p)
        .collect();
    for (at, s) in stairs.iter().filter(|(_, s)| s.active) {
        let to = other_deck(*at, s.top);
        assert!(
            entries.contains(&to),
            "the stairs at {at:?} land on an entry at {to:?}"
        );
    }
}

#[test]
fn open_floor_joins_the_air_of_two_decks() {
    let app = station();
    let atmos = &app.world().resource::<StationAtmos>().0;
    let spanning = atmos
        .zones()
        .iter()
        .filter(|z| {
            let decks: BTreeSet<i32> = z.tiles.iter().map(|p| p.z).collect();
            decks.len() > 1
        })
        .count();
    assert!(spanning > 0, "some room reaches down through Luna's open floor");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn whatever_is_flushed_down_any_bin_comes_out_of_an_outlet() {
    use rs13_classic::disposals::{DisposalBin, DisposalOutlet, InDisposal};
    use rs13_classic::objects::spawn_object;
    use rs13_simulation::atmos::{Gas, GasMix};
    use rs13_world::ObjectSpec;

    let mut app = station();
    app.world_mut().run_schedule(FixedUpdate);
    let bins: Vec<(bevy_ecs::entity::Entity, TilePos)> = app
        .world_mut()
        .query::<(bevy_ecs::entity::Entity, &GridPos, &DisposalBin)>()
        .iter(app.world())
        .map(|(e, p, _)| (e, p.0))
        .collect();
    let outlets: Vec<TilePos> = app
        .world_mut()
        .query::<(&GridPos, &DisposalOutlet)>()
        .iter(app.world())
        .map(|(p, _)| p.0)
        .collect();
    assert!(
        bins.len() > 50 && outlets.len() >= 5,
        "{} bins, {} outlets",
        bins.len(),
        outlets.len()
    );
    // A wrench in every bin, pumped up and the handle engaged.
    let mut wrenches = Vec::new();
    for (bin, at) in &bins {
        let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), *at).unwrap();
        app.world_mut()
            .entity_mut(wrench)
            .remove::<GridPos>()
            .insert(InDisposal);
        let mut air = GasMix::default();
        air.temperature = 293.0;
        air.set(Gas::Nitrogen, 30.0);
        let mut bin = app.world_mut().get_mut::<DisposalBin>(*bin).unwrap();
        bin.contents.push(wrench);
        bin.handle = true;
        bin.air = air;
        wrenches.push(wrench);
    }
    for _ in 0..120 * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let near_an_outlet = |p: TilePos| {
        outlets
            .iter()
            .any(|o| o.z == p.z && (o.x - p.x).abs() + (o.y - p.y).abs() <= 3)
    };
    for wrench in wrenches {
        let at = app.world().get::<GridPos>(wrench).map(|p| p.0);
        assert!(at.is_some_and(near_an_outlet), "a wrench ended at {at:?}");
    }
}

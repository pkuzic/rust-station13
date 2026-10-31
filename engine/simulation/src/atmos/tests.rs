use rs13_world::{Dir, TileMap, TilePos};

use super::gas::{ONE_ATMOSPHERE, T20C};
use super::*;

/// Two 2×2 rooms joined by door D; room R also has door E straight to space.
///
/// ```text
///  y=5            (space)
///  y=4  #######
///  y=3  #..D..#
///  y=2  #..#..E   (space)
///  y=1  #######
/// ```
fn two_rooms() -> (AtmosGrid, TilePos, TilePos) {
    let map = TileMap::from_ron(
        r#"(
            legend: {
                ' ': (tile: (id: "space")),
                'W': (tile: (id: "wall", solid: true)),
                '.': (tile: (id: "floor")),
            },
            rows: [
                "         ",
                " WWWWWWW ",
                " W.....W ",
                " W..W... ",
                " WWWWWWW ",
                "         ",
            ],
        )"#,
        0,
    )
    .unwrap();
    let inner = TilePos::new(4, 3, 0);
    let outer = TilePos::new(7, 2, 0);
    (AtmosGrid::new(&map, &[inner, outer]), inner, outer)
}

const LEFT: TilePos = TilePos::new(2, 2, 0);
const RIGHT: TilePos = TilePos::new(6, 3, 0);
const DT: f64 = 0.05;

fn run(grid: &mut AtmosGrid, seconds: f64) {
    for _ in 0..(seconds / DT).round() as usize {
        grid.step(DT);
    }
}

#[test]
fn doors_split_rooms_into_zones_of_standard_air() {
    let (grid, ..) = two_rooms();
    assert_eq!(grid.zones().len(), 2);
    assert_ne!(grid.zone_at(LEFT), grid.zone_at(RIGHT));
    for zone in grid.zones() {
        assert_eq!(zone.tiles.len(), 4);
        assert!((zone.pressure() - ONE_ATMOSPHERE).abs() < 1e-9);
    }
}

#[test]
fn closed_doors_hold_air() {
    let (mut grid, ..) = two_rooms();
    let before = grid.total_moles();
    run(&mut grid, 10.0);
    assert_eq!(grid.total_moles(), before);
    assert!(grid.openings().is_empty());
}

#[test]
fn opening_a_door_equalises_and_conserves_moles_and_energy() {
    let (mut grid, inner, _) = two_rooms();
    // Pump the left room to two atmospheres by heating it, so moles differ only by flow.
    let left = grid.zone_at(LEFT).unwrap();
    let extra = grid.air_at(LEFT).scaled(4.0);
    grid.add_at(LEFT, &extra);
    assert!((grid.zones()[left].pressure() - 2.0 * ONE_ATMOSPHERE).abs() < 1e-6);
    let (moles, energy) = (grid.total_moles(), grid.total_energy());

    grid.set_portal_open(inner, true);
    run(&mut grid, 30.0);
    let (pl, pr) = (grid.pressure_at(LEFT), grid.pressure_at(RIGHT));
    assert!((pl - pr).abs() < 0.5, "pressures {pl} vs {pr}");
    assert!((grid.total_moles() - moles).abs() < 1e-6);
    assert!((grid.total_energy() - energy).abs() / energy < 1e-9);
}

#[test]
fn opening_to_space_vents_the_room_and_it_gets_cold() {
    let (mut grid, _, outer) = two_rooms();
    grid.set_portal_open(outer, true);
    run(&mut grid, 1.0);
    let p1 = grid.pressure_at(RIGHT);
    assert!(p1 < ONE_ATMOSPHERE * 0.7, "after 1 s: {p1}");
    assert!(
        grid.air_at(RIGHT).temperature < T20C - 20.0,
        "expansion cools the gas"
    );
    run(&mut grid, 9.0);
    assert!(
        grid.pressure_at(RIGHT) < 1.0,
        "after 10 s: {}",
        grid.pressure_at(RIGHT)
    );
    // The sealed left room is untouched.
    assert!((grid.pressure_at(LEFT) - ONE_ATMOSPHERE).abs() < 1e-9);
}

#[test]
fn a_chain_of_open_doors_vents_both_rooms() {
    let (mut grid, inner, outer) = two_rooms();
    grid.set_portal_open(inner, true);
    grid.set_portal_open(outer, true);
    run(&mut grid, 20.0);
    assert!(grid.pressure_at(LEFT) < 1.0);
    assert!(grid.pressure_at(RIGHT) < 1.0);
}

#[test]
fn closing_the_door_stops_the_leak() {
    let (mut grid, _, outer) = two_rooms();
    grid.set_portal_open(outer, true);
    run(&mut grid, 0.5);
    grid.set_portal_open(outer, false);
    let held = grid.pressure_at(RIGHT);
    run(&mut grid, 5.0);
    assert_eq!(grid.pressure_at(RIGHT), held);
}

#[test]
fn breaching_a_wall_vents_and_conserves_until_it_does() {
    let (mut grid, ..) = two_rooms();
    let moles = grid.total_moles();
    // Knock out the left room's south wall: it now faces space below.
    grid.set_cell(TilePos::new(2, 1, 0), Cell::Open);
    grid.step(0.0);
    assert!((grid.total_moles() - moles).abs() < 1e-9, "rebuild conserves gas");
    assert_eq!(grid.zones()[grid.zone_at(LEFT).unwrap()].tiles.len(), 5);
    run(&mut grid, 15.0);
    assert!(grid.pressure_at(LEFT) < 1.0);
    assert!((grid.pressure_at(RIGHT) - ONE_ATMOSPHERE).abs() < 1e-9);
}

#[test]
fn wind_blows_towards_the_breach() {
    let (mut grid, _, outer) = two_rooms();
    grid.set_portal_open(outer, true);
    run(&mut grid, 0.2);
    let at_door = grid
        .wind_at(TilePos::new(6, 2, 0))
        .expect("wind next to the door");
    assert_eq!(at_door.dir, Dir::East);
    let far = grid.wind_at(TilePos::new(5, 3, 0)).expect("wind across the room");
    assert!(far.speed < at_door.speed);
    assert!(grid.wind_at(LEFT).is_none(), "no wind in the sealed room");
    // Near the opening the gas moves at tens of metres per second.
    assert!(at_door.speed > 10.0, "speed {}", at_door.speed);
}

#[test]
fn same_inputs_same_results() {
    let (mut a, inner, outer) = two_rooms();
    let (mut b, ..) = two_rooms();
    for grid in [&mut a, &mut b] {
        grid.set_portal_open(inner, true);
        grid.set_portal_open(outer, true);
        run(grid, 3.0);
    }
    assert_eq!(a.zones(), b.zones());
}

#[test]
fn air_at_a_door_tile_comes_from_a_neighbouring_zone() {
    let (grid, inner, _) = two_rooms();
    assert!((grid.pressure_at(inner) - ONE_ATMOSPHERE).abs() < 1e-9);
    assert_eq!(grid.pressure_at(TilePos::new(0, 0, 0)), 0.0, "space");
}

/// A 4×1 corridor with an edge barrier across its middle, between x=2 and x=3.
fn corridor_with_edge() -> AtmosGrid {
    let map = TileMap::from_ron(
        r#"(
            legend: {
                'W': (tile: (id: "wall", solid: true)),
                '.': (tile: (id: "floor")),
            },
            rows: ["WWWWWW", "W....W", "WWWWWW"],
        )"#,
        0,
    )
    .unwrap();
    let mut grid = AtmosGrid::new(&map, &[]);
    grid.add_edge(EDGE_WEST, Dir::East);
    grid.step(DT);
    grid
}

const EDGE_WEST: TilePos = TilePos::new(2, 1, 0);
const EDGE_EAST: TilePos = TilePos::new(3, 1, 0);

#[test]
fn an_edge_barrier_splits_zones() {
    let grid = corridor_with_edge();
    assert_eq!(grid.zones().len(), 2);
    assert_ne!(grid.zone_at(EDGE_WEST), grid.zone_at(EDGE_EAST));
    for zone in grid.zones() {
        assert!((zone.pressure() - ONE_ATMOSPHERE).abs() < 1e-9);
    }
}

#[test]
fn a_closed_edge_holds_a_pressure_difference() {
    let mut grid = corridor_with_edge();
    grid.set_edge_open(EDGE_EAST, Dir::West, false);
    let east = grid.air_at(EDGE_EAST).total_moles() * 2.0;
    grid.remove_at(EDGE_EAST, east);
    let before = grid.total_moles();
    run(&mut grid, 10.0);
    assert_eq!(grid.total_moles(), before);
    assert!(grid.openings().is_empty());
    assert!(grid.pressure_at(EDGE_EAST) < 1e-6);
    assert!((grid.pressure_at(EDGE_WEST) - ONE_ATMOSPHERE).abs() < 1e-9);
}

#[test]
fn an_open_edge_lets_the_sides_equalise() {
    let mut grid = corridor_with_edge();
    grid.set_edge_open(EDGE_WEST, Dir::East, false);
    let east = grid.air_at(EDGE_EAST).total_moles() * 2.0;
    grid.remove_at(EDGE_EAST, east);
    let before = grid.total_moles();
    grid.set_edge_open(EDGE_WEST, Dir::East, true);
    run(&mut grid, 60.0);
    assert!(
        (grid.total_moles() - before).abs() < 1e-9 * before,
        "moles conserved"
    );
    let (w, e) = (grid.pressure_at(EDGE_WEST), grid.pressure_at(EDGE_EAST));
    assert!((w - e).abs() < 0.5, "equalised: {w:.2} vs {e:.2}");
    assert!(e > 10.0, "air came through: {e:.2}");
}

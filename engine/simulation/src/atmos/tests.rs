use rs13_world::{Dir, TileMap, TilePos};

use super::gas::{ONE_ATMOSPHERE, T20C, TILE_VOLUME};
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

#[test]
fn hot_air_cools_toward_the_walls() {
    let (mut grid, ..) = two_rooms();
    let left = grid.zone_at(LEFT).unwrap();
    // Heat the left room by adding hot air of the same make-up.
    let mut hot = grid.air_at(LEFT);
    hot.temperature = 2000.0;
    grid.add_at(LEFT, &hot);
    let start = grid.zones()[left].mix.temperature;
    assert!(start > 500.0);
    for _ in 0..600 {
        grid.exchange_heat(0.1);
    }
    let after = grid.zones()[left].mix.temperature;
    assert!(after < start, "{start} → {after}");
    assert!(after >= T20C, "never below the walls");
    // Air at the walls' temperature stays put.
    let right = grid.zone_at(RIGHT).unwrap();
    assert!((grid.zones()[right].mix.temperature - T20C).abs() < 1e-9);
}

/// Two rooms with a grille (`g`) or a window (`w`) tile between them.
fn rooms_split_by(tile: &str) -> AtmosGrid {
    let map = TileMap::from_ron(
        &format!(
            r#"(
                legend: {{
                    'W': (tile: (id: "wall", solid: true)),
                    '.': (tile: (id: "floor")),
                    'g': (tile: (id: "grille", solid: true, opaque: Some(false), airtight: Some(false))),
                    'w': (tile: (id: "window", solid: true, opaque: Some(false))),
                }},
                rows: [
                    "WWWWWWW",
                    "W..{tile}..W",
                    "WWWWWWW",
                ],
            )"#
        ),
        0,
    )
    .unwrap();
    AtmosGrid::new(&map, &[])
}

#[test]
fn air_goes_through_a_grille_but_not_a_window() {
    let (left, right) = (TilePos::new(1, 1, 0), TilePos::new(5, 1, 0));
    let grille = rooms_split_by("g");
    assert_eq!(grille.zone_at(left), grille.zone_at(right), "one room of air");
    let window = rooms_split_by("w");
    assert_ne!(window.zone_at(left), window.zone_at(right), "two rooms of air");
}

/// A 20 × 6 room with `plasma` moles per tile spread through its air, and a sealed room of
/// the same size east of it, behind a wall.
fn big_rooms(plasma_per_tile: f64) -> AtmosGrid {
    let row = format!("W{}W{}W", ".".repeat(20), ".".repeat(20));
    let wall = "W".repeat(43);
    let mut rows = vec![wall.clone()];
    rows.extend(std::iter::repeat_n(row, 6));
    rows.push(wall);
    let rows: Vec<String> = rows.into_iter().map(|r| format!("\"{r}\"")).collect();
    let map = TileMap::from_ron(
        &format!(
            r#"(
                legend: {{
                    'W': (tile: (id: "wall", solid: true)),
                    '.': (tile: (id: "floor")),
                }},
                rows: [{}],
            )"#,
            rows.join(", ")
        ),
        0,
    )
    .unwrap();
    let mut grid = AtmosGrid::new(&map, &[]);
    for z in 0..grid.zones().len() {
        let tiles = grid.zones()[z].tiles.len() as f64;
        let at = grid.zones()[z].tiles[0];
        let mut gas = GasMix {
            temperature: T20C,
            ..GasMix::default()
        };
        gas.set(Gas::Plasma, plasma_per_tile * tiles);
        grid.add_at(at, &gas);
    }
    grid
}

fn burning_in(grid: &AtmosGrid, x_range: std::ops::Range<i32>) -> usize {
    grid.burning_tiles()
        .iter()
        .filter(|(p, _)| x_range.contains(&p.x))
        .count()
}

#[test]
fn a_flame_lights_one_tile_of_a_big_room_and_the_fire_spreads_across_it() {
    let mut grid = big_rooms(2.0);
    let corner = TilePos::new(1, 1, 0);
    let before = grid.total_moles();
    assert!(grid.expose(corner, 3500.0) > 0.0, "the flame lights it");
    assert!(
        (grid.total_moles() - before).abs() < 1e-9,
        "the cell's air came from its zone"
    );
    assert_eq!(grid.fire_cells().count(), 1);

    run(&mut grid, 1.0);
    let near = burning_in(&grid, 1..4);
    let far = burning_in(&grid, 12..21);
    assert!(near > 0 && far == 0, "first the corner: {near} near, {far} far");

    // About two tiles a second: across the 20-tile room in well under half a minute.
    let mut reached_far = None;
    for second in 1..40 {
        run(&mut grid, 1.0);
        if burning_in(&grid, 18..21) > 0 {
            reached_far = Some(second);
            break;
        }
    }
    let second = reached_far.expect("the fire crossed the room");
    assert!(second >= 4, "tile by tile, not all at once: {second} s");
    assert_eq!(burning_in(&grid, 22..42), 0, "not through the wall");
}

#[test]
fn burnt_out_cells_go_back_into_their_zone() {
    let mut grid = big_rooms(0.6);
    grid.expose(TilePos::new(1, 1, 0), 3500.0);
    run(&mut grid, 120.0);
    assert_eq!(grid.fire_cells().count(), 0, "all settled back");
    let zone = &grid.zones()[grid.zone_at(TilePos::new(1, 1, 0)).unwrap()];
    assert_eq!(zone.volume, zone.tiles.len() as f64 * TILE_VOLUME);
    assert!(grid.burning_tiles().is_empty());
}

#[test]
fn without_enough_plasma_on_the_tile_nothing_lights() {
    let mut grid = big_rooms(0.3);
    assert_eq!(grid.expose(TilePos::new(1, 1, 0), 3500.0), 0.0);
    assert_eq!(grid.fire_cells().count(), 0);
}

#[test]
fn a_burning_room_opened_to_space_takes_its_fire_cells_with_it() {
    // The right room of `two_rooms` has a door straight to space.
    let (mut grid, _, outer) = two_rooms();
    let mut gas = GasMix {
        temperature: T20C,
        ..GasMix::default()
    };
    gas.set(Gas::Plasma, 8.0);
    grid.add_at(RIGHT, &gas);
    assert!(grid.expose(RIGHT, 3500.0) > 0.0);
    run(&mut grid, 0.5);
    assert!(grid.fire_cells().count() > 0);
    grid.set_portal_open(outer, true);
    run(&mut grid, 1.0);
    assert_eq!(grid.fire_cells().count(), 0, "they vent with the room");
}

#[test]
fn decks_keep_their_own_air() {
    let deck = |z: i32| {
        TileMap::from_ron(
            r#"(
                legend: {
                    ' ': (tile: (id: "space")),
                    'W': (tile: (id: "wall", solid: true)),
                    '.': (tile: (id: "floor")),
                },
                rows: [
                    "WWWW",
                    "W.. ",
                    "WWWW",
                ],
            )"#,
            z,
        )
        .unwrap()
    };
    let (lower, upper) = (deck(0), deck(1));
    // The lower deck's room is open to space at its east end; the upper one's door there
    // is shut.
    let door = TilePos::new(3, 1, 1);
    let mut grid = AtmosGrid::new_decks(&[&lower, &upper], &[door]);
    let (low, high) = (TilePos::new(1, 1, 0), TilePos::new(1, 1, 1));
    assert_ne!(grid.zone_at(low), grid.zone_at(high));
    assert!((grid.pressure_at(high) - ONE_ATMOSPHERE).abs() < 1e-6);
    run(&mut grid, 20.0);
    assert!(grid.pressure_at(low) < 1.0, "the breached deck vents");
    assert!(
        (grid.pressure_at(high) - ONE_ATMOSPHERE).abs() < 1e-6,
        "the other keeps its air"
    );
}

#[test]
fn air_goes_down_a_hole_in_the_floor() {
    let deck = |z: i32, hole: bool| {
        let middle = if hole { "W.oW" } else { "W..W" };
        TileMap::from_ron(
            &format!(
                r#"(
                    legend: {{
                        'W': (tile: (id: "wall", solid: true)),
                        '.': (tile: (id: "floor")),
                        'o': (tile: (id: "open-space")),
                    }},
                    rows: ["WWWW", "{middle}", "WWWW"],
                )"#
            ),
            z,
        )
        .unwrap()
    };
    let (lower, upper) = (deck(0, false), deck(1, true));
    let grid = AtmosGrid::new_decks(&[&lower, &upper], &[]);
    let (down, up) = (TilePos::new(1, 1, 0), TilePos::new(1, 1, 1));
    assert_eq!(
        grid.zone_at(down),
        grid.zone_at(up),
        "one room of air through the hole"
    );
    let sealed = AtmosGrid::new_decks(&[&lower, &deck(1, false)], &[]);
    assert_ne!(
        sealed.zone_at(down),
        sealed.zone_at(up),
        "but not through a whole floor"
    );
}

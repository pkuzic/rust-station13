//! Scenario: two mobs move in the engineering chamber at Luna's speed and never overlap.

use bevy_app::{App, FixedUpdate};
use rs13_classic::movement::{Mob, MoveIntent};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Dir, GridPos, MapLayer, TileMap, TilePos};

fn setup() -> (App, Vec<TilePos>) {
    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap();
    let spawns = map.spawns.clone();
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    app.world_mut().spawn(MapLayer(map));
    (app, spawns)
}

fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn pos(app: &App, e: bevy_ecs::entity::Entity) -> TilePos {
    app.world().get::<GridPos>(e).unwrap().0
}

#[test]
fn running_mob_steps_every_five_ticks() {
    let (mut app, spawns) = setup();
    let start = spawns[0];
    let mob = app
        .world_mut()
        .spawn((
            GridPos(start),
            Mob::default(),
            MoveIntent {
                dir: Some(Dir::East),
                run: true,
            },
        ))
        .id();

    // First step on the first tick, then one step per 2.5 ds (5 ticks at 20 TPS).
    tick(&mut app, 1);
    assert_eq!(pos(&app, mob).x, start.x + 1);
    tick(&mut app, 4);
    assert_eq!(pos(&app, mob).x, start.x + 1);
    tick(&mut app, 1);
    assert_eq!(pos(&app, mob).x, start.x + 2);
    tick(&mut app, 10);
    assert_eq!(pos(&app, mob).x, start.x + 4);
}

#[test]
fn two_mobs_walk_into_each_other_and_stop() {
    let (mut app, spawns) = setup();
    // Two spawns on the same row face each other across the room.
    let left = spawns[0];
    let right = *spawns.iter().find(|s| s.y == left.y && s.x > left.x).unwrap();
    let a = app
        .world_mut()
        .spawn((
            GridPos(left),
            Mob::default(),
            MoveIntent {
                dir: Some(Dir::East),
                run: true,
            },
        ))
        .id();
    let b = app
        .world_mut()
        .spawn((
            GridPos(right),
            Mob::default(),
            MoveIntent {
                dir: Some(Dir::West),
                run: true,
            },
        ))
        .id();

    tick(&mut app, 200);
    let (pa, pb) = (pos(&app, a), pos(&app, b));
    assert_ne!(pa, pb);
    assert_eq!(pa.y, left.y);
    assert_eq!(pb.x - pa.x, 1, "they should end up adjacent, not overlapping");
}

#[test]
fn mobs_stay_inside_the_walls() {
    let (mut app, spawns) = setup();
    let mob = app
        .world_mut()
        .spawn((
            GridPos(spawns[0]),
            Mob::default(),
            MoveIntent {
                dir: Some(Dir::SouthWest),
                run: true,
            },
        ))
        .id();
    tick(&mut app, 200);
    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap();
    let p = pos(&app, mob);
    assert!(!map.is_solid(p));
    // Slid into the bottom-left corner of the room.
    assert_eq!(map.tile(p.step(Dir::West)).unwrap().id, "wall");
    assert_eq!(map.tile(p.step(Dir::South)).unwrap().id, "wall");
}

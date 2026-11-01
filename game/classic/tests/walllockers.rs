//! Emergency wall lockers. Spec: docs/reference/walllockers.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::walllockers::{AMOUNT, WallLocker};
use rs13_world::{Appearance, GridPos, PixelOffset, TileMap, TilePos};

/// A locker hung on the north wall above (2, 1), the way Luna's `emerglocker/north` is.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'l': (tile: (id: "floor"), objects: [(id: "emergency-locker", dir: South, vars: {"pixel_y": "32"})]),
    },
    rows: [
        "WWWWW",
        "W.l.W",
        "W...W",
        "WWWWW",
    ],
)"#;

const LOCKER_AT: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let locker = app
        .world_mut()
        .query_filtered::<Entity, With<WallLocker>>()
        .single(app.world())
        .unwrap();
    (app, locker)
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn on_tile(app: &mut App, at: TilePos) -> Vec<String> {
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    let mut out: Vec<String> = q
        .iter(app.world())
        .filter(|(_, p)| p.0 == at)
        .map(|(a, _)| a.sprite.clone())
        .collect();
    out.sort();
    out
}

#[test]
fn it_hands_out_one_of_each_three_times_then_is_empty() {
    let (mut app, locker) = setup();
    assert_eq!(
        app.world().get::<PixelOffset>(locker),
        Some(&PixelOffset([0, 32]))
    );
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    for round in 1..=AMOUNT {
        assert_eq!(click(&mut app, mob, locker), ["walllocker-dispensed-3"]);
        let things = on_tile(&mut app, LOCKER_AT);
        assert_eq!(things.iter().filter(|s| *s == "crowbar").count(), round);
        assert_eq!(things.iter().filter(|s| *s == "breath-mask").count(), round);
        assert_eq!(things.iter().filter(|s| *s == "emergency-oxygen").count(), round);
    }
    assert_eq!(click(&mut app, mob, locker), ["walllocker-empty"]);
}

#[test]
fn what_came_out_goes_back_in() {
    let (mut app, locker) = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    click(&mut app, mob, locker);
    let crowbar = {
        let mut q = app.world_mut().query::<(Entity, &Appearance, &GridPos)>();
        q.iter(app.world())
            .find(|(_, a, p)| a.sprite == "crowbar" && p.0 == LOCKER_AT)
            .unwrap()
            .0
    };
    click(&mut app, mob, crowbar);
    assert_eq!(
        app.world().get::<Hands>(mob).unwrap().active_item(),
        Some(crowbar)
    );
    assert_eq!(click(&mut app, mob, locker), ["walllocker-put-back"]);
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);
    assert_eq!(
        app.world().get::<WallLocker>(locker).unwrap().stock[0].len(),
        AMOUNT
    );

    // A full locker takes nothing more of that kind.
    let another = rs13_classic::objects::spawn_object(
        app.world_mut(),
        &rs13_world::ObjectSpec::new("crowbar"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    click(&mut app, mob, another);
    assert!(click(&mut app, mob, locker).is_empty());
    assert_eq!(
        app.world().get::<Hands>(mob).unwrap().active_item(),
        Some(another)
    );
}

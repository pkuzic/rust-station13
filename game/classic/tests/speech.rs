//! Scenarios for speech: who hears what. Spec: docs/reference/speech.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::Outbox;
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::speech::PendingSpeech;
use rs13_classic::{ClassicPlugin, maps};
use rs13_i18n::LocArg;
use rs13_world::{Appearance, DisplayName, TileMap, TilePos};

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    app
}

fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn mob(app: &mut App, x: i32, y: i32, name: &str) -> Entity {
    let e = spawn_player_mob(app.world_mut(), TilePos::new(x, y, 0));
    app.world_mut().get_mut::<DisplayName>(e).unwrap().0 = name.to_owned();
    e
}

fn say(app: &mut App, who: Entity, text: &str) {
    app.world_mut()
        .get_mut::<PendingSpeech>(who)
        .unwrap()
        .0
        .push(text.to_owned());
    tick(app, 1);
}

/// The `text` argument of every `say` message this mob has received, then clear.
fn heard(app: &mut App, who: Entity) -> Vec<String> {
    let mut outbox = app.world_mut().get_mut::<Outbox>(who).unwrap();
    let lines = outbox
        .0
        .iter()
        .filter(|m| m.id == "say")
        .filter_map(|m| m.args.iter().find(|(k, _)| k == "text"))
        .map(|(_, v)| match v {
            LocArg::Str(s) => s.clone(),
            LocArg::Num(n) => n.to_string(),
        })
        .collect();
    outbox.0.clear();
    lines
}

fn airlock(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query::<(Entity, &Appearance, &rs13_world::GridPos)>();
    q.iter(app.world())
        .find(|(_, a, p)| a.sprite == "airlock" && p.0 == TilePos::new(16, 6, 0))
        .unwrap()
        .0
}

#[test]
fn people_in_sight_hear_and_the_speaker_hears_themselves() {
    let mut app = setup();
    let alice = mob(&mut app, 3, 3, "Alice Abbott");
    let bob = mob(&mut app, 8, 4, "Bob Barnes");
    say(&mut app, alice, "  hello bob ");
    assert_eq!(heard(&mut app, alice), ["Hello bob"]);
    assert_eq!(heard(&mut app, bob), ["Hello bob"]);

    // Name and verb travel with the words.
    say(&mut app, bob, "who's there?");
    let outbox = &app.world().get::<Outbox>(alice).unwrap().0;
    let msg = outbox.last().unwrap();
    assert!(
        msg.args
            .contains(&("name".into(), LocArg::Str("Bob Barnes".into())))
    );
    assert!(msg.args.contains(&("verb".into(), LocArg::Str("ask".into()))));
}

#[test]
fn speech_carries_seven_tiles_and_no_further() {
    let mut app = setup();
    let speaker = mob(&mut app, 2, 9, "Alice Abbott");
    let near = mob(&mut app, 9, 9, "Bob Barnes");
    let far = mob(&mut app, 10, 9, "Carl Carver");
    say(&mut app, speaker, "Testing.");
    assert_eq!(heard(&mut app, near).len(), 1);
    assert!(heard(&mut app, far).is_empty());
}

#[test]
fn walls_and_closed_airlocks_block_speech_but_an_open_airlock_does_not() {
    let mut app = setup();
    // Either side of the airlock at (16, 6), in line with the doorway.
    let inside = mob(&mut app, 14, 6, "Alice Abbott");
    let closet = mob(&mut app, 18, 6, "Bob Barnes");
    // Behind the wall, off the doorway line.
    let walled = mob(&mut app, 18, 9, "Carl Carver");

    say(&mut app, inside, "Can you hear me?");
    assert!(
        heard(&mut app, closet).is_empty(),
        "the closed airlock blocks sight"
    );
    assert!(heard(&mut app, walled).is_empty());

    let door = airlock(&mut app);
    app.world_mut().get_mut::<Airlock>(door).unwrap().state = DoorState::Open { autoclose_at: None };
    tick(&mut app, 1);
    say(&mut app, inside, "Now?");
    assert_eq!(heard(&mut app, closet), ["Now?"]);
    assert!(heard(&mut app, walled).is_empty(), "walls still block");
}

#[test]
fn empty_speech_is_ignored() {
    let mut app = setup();
    let alice = mob(&mut app, 3, 3, "Alice Abbott");
    say(&mut app, alice, "   \n ");
    assert!(heard(&mut app, alice).is_empty());
}

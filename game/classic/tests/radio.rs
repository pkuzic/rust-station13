//! Radios and intercoms. Spec: docs/reference/radio.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::radio::Radio;
use rs13_classic::speech::PendingSpeech;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A long hall with an intercom at each end.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'I': (tile: (id: "floor"), objects: ["intercom"]),
    },
    rows: [
        "WWWWWWWWWWWWWWWWWWWWWWWW",
        "WI....................IW",
        "WWWWWWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

fn at(x: i32) -> TilePos {
    TilePos::new(x, 1, 0)
}

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn mob(app: &mut App, x: i32) -> Entity {
    spawn_player_mob(app.world_mut(), at(x))
}

fn give_radio(app: &mut App, mob: Entity) -> Entity {
    let radio = spawn_object(app.world_mut(), &ObjectSpec::new("radio"), at(1)).unwrap();
    app.world_mut()
        .entity_mut(radio)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(radio);
    radio
}

fn say(app: &mut App, mob: Entity, text: &str) {
    app.world_mut()
        .get_mut::<PendingSpeech>(mob)
        .unwrap()
        .0
        .push(text.to_owned());
    app.world_mut().run_schedule(FixedUpdate);
}

fn heard(app: &App, mob: Entity) -> Vec<LocMsg> {
    app.world().get::<Outbox>(mob).unwrap().0.clone()
}

fn ids(app: &App, mob: Entity) -> Vec<String> {
    heard(app, mob).into_iter().map(|m| m.id).collect()
}

#[test]
fn talking_into_a_radio_reaches_other_radios_on_the_channel() {
    let mut app = setup();
    let speaker = mob(&mut app, 5);
    let beside = mob(&mut app, 6);
    let near = mob(&mut app, 9);
    let far = mob(&mut app, 18);
    let far_neighbour = mob(&mut app, 19);
    give_radio(&mut app, speaker);
    give_radio(&mut app, far);
    app.world_mut().run_schedule(FixedUpdate);

    say(&mut app, speaker, ":r hello there");
    // Heard directly only next to you, and from your own radio too (Luna sends to every
    // radio on the channel, the speaker's included).
    assert_eq!(ids(&app, speaker), ["say", "radio-say"]);
    assert_eq!(ids(&app, beside), ["say", "radio-say"]);
    assert!(
        ids(&app, near).is_empty(),
        "four tiles off, beyond the west intercom, is too far"
    );
    assert_eq!(ids(&app, far), ["radio-say"]);
    assert_eq!(
        ids(&app, far_neighbour),
        ["radio-say"],
        "a radio is heard one tile off"
    );
    let msg = &heard(&app, far)[0];
    assert!(format!("{msg:?}").contains("145.9"), "{msg:?}");
    assert!(format!("{msg:?}").contains("Hello there"), "{msg:?}");
}

#[test]
fn intercoms_carry_speech_down_the_hall() {
    let mut app = setup();
    let speaker = mob(&mut app, 2);
    let listener = mob(&mut app, 16);
    app.world_mut().run_schedule(FixedUpdate);

    // Plain speech doesn't reach 14 tiles.
    say(&mut app, speaker, "anyone?");
    assert!(ids(&app, listener).is_empty());

    // Into the intercom next to you: the far intercom plays it within 7 tiles.
    say(&mut app, speaker, ":i anyone?");
    assert_eq!(ids(&app, listener), ["radio-say"]);
}

#[test]
fn an_intercom_with_its_microphone_on_passes_on_what_it_hears() {
    let mut app = setup();
    let speaker = mob(&mut app, 4);
    let listener = mob(&mut app, 18);
    app.world_mut().run_schedule(FixedUpdate);
    let near_intercom = {
        let mut q = app.world_mut().query::<(Entity, &GridPos, &Appearance)>();
        q.iter(app.world())
            .find(|(_, p, a)| a.sprite == "intercom" && p.0 == at(1))
            .unwrap()
            .0
    };

    // An empty hand switches the microphone on, from next to it.
    let operator = mob(&mut app, 2);
    app.world_mut()
        .get_mut::<PendingActions>(operator)
        .unwrap()
        .0
        .push(Action::UseOn(near_intercom));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Radio>(near_intercom).unwrap().broadcasting);
    assert!(ids(&app, operator).contains(&"radio-microphone-on".to_owned()));

    say(&mut app, speaker, "can you hear me");
    assert_eq!(
        ids(&app, listener),
        ["radio-say"],
        "said three tiles from an open microphone"
    );
}

fn wear_headset(app: &mut App, mob: Entity) {
    let headset = spawn_object(app.world_mut(), &ObjectSpec::new("headset-eng"), at(1)).unwrap();
    app.world_mut()
        .entity_mut(headset)
        .remove::<GridPos>()
        .insert(rs13_classic::equipment::WornBy(mob));
    app.world_mut()
        .get_mut::<rs13_classic::equipment::Equipment>(mob)
        .unwrap()
        .0
        .insert(rs13_world::Slot::Ears, headset);
}

#[test]
fn a_semicolon_talks_into_the_headset() {
    let mut app = setup();
    let speaker = mob(&mut app, 9);
    let listener = mob(&mut app, 13);
    wear_headset(&mut app, speaker);
    wear_headset(&mut app, listener);
    app.world_mut().run_schedule(FixedUpdate);
    say(&mut app, speaker, "; engineering, report");
    assert_eq!(
        ids(&app, listener),
        ["radio-say"],
        "four tiles off, heard on the headset"
    );
    assert!(format!("{:?}", heard(&app, listener)[0]).contains("Engineering, report"));
}

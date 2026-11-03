//! Department channels on headsets. Spec: docs/reference/radio.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::jobs::jobs;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::radio::headset_channels;
use rs13_classic::speech::PendingSpeech;
use rs13_i18n::LocArg;
use rs13_world::{GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: [
        "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
        "W............................W",
        "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn wearing(app: &mut App, x: i32, headset: &str) -> Entity {
    let at = TilePos::new(x, 1, 0);
    let mob = spawn_player_mob(app.world_mut(), at);
    let ear = spawn_object(app.world_mut(), &ObjectSpec::new(headset), at).unwrap();
    app.world_mut()
        .entity_mut(ear)
        .remove::<GridPos>()
        .insert(WornBy(mob));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(Slot::Ears, ear);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().get_mut::<Outbox>(mob).unwrap().0.clear();
    mob
}

fn say(app: &mut App, who: Entity, text: &str) {
    app.world_mut()
        .get_mut::<PendingSpeech>(who)
        .unwrap()
        .0
        .push(text.to_owned());
    app.world_mut().run_schedule(FixedUpdate);
}

/// What came over the radio to this mob: its channel or frequency, as sent.
fn heard(app: &mut App, who: Entity) -> Vec<LocArg> {
    let mut outbox = app.world_mut().get_mut::<Outbox>(who).unwrap();
    let lines = outbox
        .0
        .iter()
        .filter(|m| m.id == "radio-say")
        .filter_map(|m| {
            m.args
                .iter()
                .find(|(k, _)| k == "frequency")
                .map(|(_, v)| v.clone())
        })
        .collect();
    outbox.0.clear();
    lines
}

fn channel(id: &str) -> LocArg {
    LocArg::Msg(format!("channel-{id}"))
}

#[test]
fn a_channel_reaches_only_the_headsets_with_it() {
    let mut app = setup();
    let engineer = wearing(&mut app, 1, "headset-eng");
    let other_engineer = wearing(&mut app, 10, "headset-eng");
    let medic = wearing(&mut app, 20, "headset-med");
    let chief = wearing(&mut app, 25, "headset-ce");

    say(&mut app, engineer, ":e the engine is on fire");
    assert_eq!(heard(&mut app, engineer), [channel("engineering")]);
    assert_eq!(heard(&mut app, other_engineer), [channel("engineering")]);
    assert_eq!(heard(&mut app, chief), [channel("engineering")]);
    assert!(heard(&mut app, medic).is_empty());

    // `:h` is the headset's first, and a Russian keyboard's key does the same as `:e`.
    say(&mut app, chief, ":h and again");
    assert_eq!(heard(&mut app, other_engineer), [channel("engineering")]);
    say(&mut app, chief, ":у and again");
    assert_eq!(heard(&mut app, other_engineer), [channel("engineering")]);

    // `;` is still the common channel, which everyone hears.
    heard(&mut app, engineer);
    say(&mut app, medic, "; hello");
    assert_eq!(heard(&mut app, engineer), [LocArg::Str("145.9".to_owned())]);
}

#[test]
fn a_channel_your_headset_lacks_goes_nowhere() {
    let mut app = setup();
    let medic = wearing(&mut app, 1, "headset-med");
    let engineer = wearing(&mut app, 20, "headset-eng");
    say(&mut app, medic, ":e let me in");
    assert!(heard(&mut app, engineer).is_empty());
    assert!(heard(&mut app, medic).is_empty());
}

#[test]
fn the_captain_can_talk_on_channels_it_doesnt_listen_to() {
    let mut app = setup();
    let captain = wearing(&mut app, 1, "headset-captain");
    let engineer = wearing(&mut app, 20, "headset-eng");
    say(&mut app, captain, ":e status report");
    assert_eq!(heard(&mut app, engineer), [channel("engineering")]);
    assert!(heard(&mut app, captain).is_empty(), "Luna starts that one off");
    say(&mut app, engineer, ":e all fine");
    assert!(heard(&mut app, captain).is_empty());
}

#[test]
fn every_job_wears_a_headset() {
    for job in jobs() {
        let worn: Vec<&String> = job
            .wears
            .iter()
            .filter(|w| headset_channels(w).is_some())
            .collect();
        assert_eq!(worn.len(), 1, "{}: {:?}", job.id, job.wears);
    }
}

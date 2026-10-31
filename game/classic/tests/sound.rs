//! Positional sound: who hears what, and how walls, doors and vacuum change it.
//! Spec: docs/reference/sound.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::health::{Body, Vitals, Zone};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::sound::{
    AIRLOCK, CLOSET_CLICK, CONTACT_PASS, DISTANCE_FALLOFF, DOOR_DENY, Earshot, OBSTRUCTION_PASS, SCREWDRIVER,
    SOUND_RANGE, SoundBarriers, SoundEvent, SoundEvents,
};
use rs13_world::{GridPos, MobStat, TileMap, TilePos};

/// Two rooms joined by an airlock, with a closet and a screwdriver in the west room and
/// space to the east of the station.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'A': (tile: (id: "floor"), objects: ["airlock"]),
        'c': (tile: (id: "floor"), objects: ["closet"]),
        's': (tile: (id: "floor"), objects: ["screwdriver"]),
    },
    rows: [
        "WWWWWWWWWWWWW    ",
        "W.c...W.....W    ",
        "W...s.A.....W    ",
        "W.....W.....W    ",
        "WWWWWWWWWWWWW    ",
    ],
)"#;

const DOOR: TilePos = TilePos::new(6, 2, 0);
const CLOSET: TilePos = TilePos::new(2, 3, 0);
const SCREWDRIVER_AT: TilePos = TilePos::new(4, 2, 0);
const SPACE: TilePos = TilePos::new(14, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    ticks(&mut app, 2);
    app
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn mob_at(app: &mut App, pos: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), pos);
    ticks(app, 1);
    hear(app, mob);
    mob
}

/// What `mob` heard since last asked: (sound, volume).
fn hear(app: &mut App, mob: Entity) -> Vec<(&'static str, f32)> {
    let mut earshot = app.world_mut().get_mut::<Earshot>(mob).unwrap();
    earshot.0.drain(..).map(|h| (h.sound, h.volume)).collect()
}

/// Make a sound and let it be delivered.
fn play(app: &mut App, sound: SoundEvent) {
    app.world_mut().resource_mut::<SoundEvents>().0.push(sound);
    ticks(app, 1);
}

fn volume(app: &mut App, mob: Entity) -> Option<f32> {
    let heard = hear(app, mob);
    assert!(heard.len() <= 1, "{heard:?}");
    heard.first().map(|(_, v)| *v)
}

/// Volume of a sound at 100 in open air `tiles` tiles away, before obstructions.
fn open_air(tiles: f32) -> f32 {
    100.0 * (1.0 - DISTANCE_FALLOFF * tiles / SOUND_RANGE as f32)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn door(app: &mut App) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Airlock)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == DOOR).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(target);
}

fn entity_at(app: &mut App, pos: TilePos, skip: Entity) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos)>();
    q.iter(app.world())
        .find(|(e, p)| p.0 == pos && *e != skip)
        .map(|(e, _)| e)
        .unwrap()
}

#[test]
fn near_mobs_hear_and_far_ones_do_not() {
    let mut app = setup();
    let near = mob_at(&mut app, TilePos::new(3, 2, 0));
    let far = mob_at(&mut app, TilePos::new(11, 2, 0));
    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(1, 2, 0), 100.0));
    assert!(close(volume(&mut app, near).unwrap(), open_air(2.0)));
    assert_eq!(volume(&mut app, far), None);
}

#[test]
fn the_excluded_and_the_unconscious_hear_nothing() {
    let mut app = setup();
    let maker = mob_at(&mut app, TilePos::new(2, 2, 0));
    let sleeper = mob_at(&mut app, TilePos::new(3, 2, 0));
    app.world_mut()
        .get_mut::<Body>(sleeper)
        .unwrap()
        .take_damage(Zone::Chest, 101.0, 0.0);
    ticks(&mut app, 1);
    assert_eq!(
        app.world().get::<Vitals>(sleeper).unwrap().stat,
        MobStat::Unconscious
    );
    let mut sound = SoundEvent::new(AIRLOCK, TilePos::new(2, 2, 0), 100.0);
    sound.except = Some(maker);
    play(&mut app, sound);
    assert_eq!(volume(&mut app, maker), None);
    assert_eq!(volume(&mut app, sleeper), None);
}

#[test]
fn a_wall_and_a_closed_door_muffle_alike_and_an_open_door_does_not() {
    let mut app = setup();
    // Four tiles east of each source, behind the wall and behind the door.
    let behind_wall = mob_at(&mut app, TilePos::new(8, 3, 0));
    let behind_door = mob_at(&mut app, TilePos::new(8, 2, 0));
    let muffled = open_air(4.0) * OBSTRUCTION_PASS;

    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(4, 3, 0), 100.0));
    assert!(close(volume(&mut app, behind_wall).unwrap(), muffled));
    hear(&mut app, behind_door);

    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(4, 2, 0), 100.0));
    assert!(close(volume(&mut app, behind_door).unwrap(), muffled));
    hear(&mut app, behind_wall);

    let door = door(&mut app);
    app.world_mut().get_mut::<Airlock>(door).unwrap().state = DoorState::Open { autoclose_at: None };
    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(4, 2, 0), 100.0));
    assert!(close(volume(&mut app, behind_door).unwrap(), open_air(4.0)));
}

#[test]
fn a_closed_edge_muffles_like_a_wall() {
    let mut app = setup();
    let listener = mob_at(&mut app, TilePos::new(10, 2, 0));
    app.world_mut()
        .resource_mut::<SoundBarriers>()
        .0
        .insert((TilePos::new(9, 2, 0), TilePos::new(8, 2, 0)));
    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(7, 2, 0), 100.0));
    assert!(close(
        volume(&mut app, listener).unwrap(),
        open_air(3.0) * OBSTRUCTION_PASS
    ));
}

#[test]
fn a_sound_inside_is_not_heard_in_space_outside() {
    let mut app = setup();
    // The station's east wall, then vacuum: nothing gets out.
    let outside = mob_at(&mut app, SPACE);
    play(&mut app, SoundEvent::new(AIRLOCK, TilePos::new(11, 2, 0), 100.0));
    assert_eq!(volume(&mut app, outside), None);
}

#[test]
fn nothing_carries_through_vacuum_but_contact_does() {
    let mut app = setup();
    let listener = mob_at(&mut app, SPACE);
    play(
        &mut app,
        SoundEvent::new(AIRLOCK, SPACE.step(rs13_world::Dir::North), 100.0),
    );
    assert_eq!(volume(&mut app, listener), None);
    play(&mut app, SoundEvent::new(AIRLOCK, SPACE, 100.0));
    assert!(close(volume(&mut app, listener).unwrap(), 100.0 * CONTACT_PASS));
}

#[test]
fn airlocks_sound_when_they_open_close_and_refuse() {
    let mut app = setup();
    let mob = mob_at(&mut app, TilePos::new(5, 2, 0));
    let listener = mob_at(&mut app, TilePos::new(9, 2, 0));
    let door = door(&mut app);
    click(&mut app, mob, Action::UseOn(door));
    ticks(&mut app, 1);
    assert_eq!(hear(&mut app, mob).first().map(|h| h.0), Some(AIRLOCK));
    // A door does not muffle its own sound.
    assert!(hear(&mut app, listener).iter().any(|h| h.0 == AIRLOCK));
    // Fully open, then it closes by itself: once more.
    ticks(&mut app, 20 * 16);
    let heard = hear(&mut app, mob);
    assert_eq!(heard.iter().filter(|h| h.0 == AIRLOCK).count(), 1, "{heard:?}");

    ticks(&mut app, 20);
    app.world_mut().get_mut::<Airlock>(door).unwrap().req_access = vec!["engine".to_owned()];
    click(&mut app, mob, Action::UseOn(door));
    ticks(&mut app, 1);
    assert_eq!(hear(&mut app, mob).first().map(|h| h.0), Some(DOOR_DENY));
}

#[test]
fn closets_click_and_tools_make_their_sound() {
    let mut app = setup();
    let mob = mob_at(&mut app, TilePos::new(3, 2, 0));
    let closet = entity_at(&mut app, CLOSET, mob);
    click(&mut app, mob, Action::UseOn(closet));
    ticks(&mut app, 1);
    assert_eq!(hear(&mut app, mob).first().map(|h| h.0), Some(CLOSET_CLICK));

    // Walk to the door with the screwdriver and open its panel.
    let screwdriver = entity_at(&mut app, SCREWDRIVER_AT, mob);
    click(&mut app, mob, Action::UseOn(screwdriver));
    ticks(&mut app, 1);
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(5, 2, 0);
    let door = door(&mut app);
    click(&mut app, mob, Action::UseOn(door));
    ticks(&mut app, 1);
    assert_eq!(hear(&mut app, mob).first().map(|h| h.0), Some(SCREWDRIVER));
    assert!(app.world().get::<Airlock>(door).unwrap().panel_open);
}

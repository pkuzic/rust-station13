//! Scenarios for hitting people, unconsciousness and death. Spec: docs/reference/health.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Vitals, Zone};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::rng::GameRng;
use rs13_classic::speech::PendingSpeech;
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, Dir, GridPos, Intent, MobStat, TileMap, TilePos};

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    app.insert_resource(GameRng::seeded(1234));
    app
}

fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn give(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

fn health(app: &App, mob: Entity) -> f32 {
    app.world().get::<Body>(mob).unwrap().health()
}

fn stat(app: &App, mob: Entity) -> MobStat {
    app.world().get::<Vitals>(mob).unwrap().stat
}

fn ids(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

#[test]
fn hitting_with_an_item_deals_its_force_on_any_intent() {
    let mut app = setup();
    let alice = spawn_player_mob(app.world_mut(), TilePos::new(5, 4, 0));
    let bob = spawn_player_mob(app.world_mut(), TilePos::new(6, 4, 0));
    let watcher = spawn_player_mob(app.world_mut(), TilePos::new(9, 4, 0));
    let crowbar = find(&mut app, "crowbar");
    give(&mut app, alice, crowbar);

    // Help intent (the default): a held item still hits.
    act(&mut app, alice, Action::UseOn(bob));
    assert_eq!(health(&app, bob), 95.0);
    for mob in [alice, bob, watcher] {
        assert_eq!(ids(&app, mob), ["attacked-with"], "everyone in sight sees it");
    }
}

#[test]
fn out_of_reach_nothing_happens() {
    let mut app = setup();
    let alice = spawn_player_mob(app.world_mut(), TilePos::new(5, 4, 0));
    let bob = spawn_player_mob(app.world_mut(), TilePos::new(7, 4, 0));
    let crowbar = find(&mut app, "crowbar");
    give(&mut app, alice, crowbar);
    act(&mut app, alice, Action::UseOn(bob));
    assert_eq!(health(&app, bob), 100.0);
    assert!(ids(&app, bob).is_empty());
}

#[test]
fn empty_hand_needs_harm_intent_to_punch() {
    let mut app = setup();
    let alice = spawn_player_mob(app.world_mut(), TilePos::new(5, 4, 0));
    let bob = spawn_player_mob(app.world_mut(), TilePos::new(6, 4, 0));

    act(&mut app, alice, Action::UseOn(bob));
    assert_eq!(health(&app, bob), 100.0, "help intent does not punch");

    act(&mut app, alice, Action::SetIntent(Intent::Harm));
    for _ in 0..50 {
        let before = health(&app, bob);
        act(&mut app, alice, Action::UseOn(bob));
        let dealt = before - health(&app, bob);
        // At most 9; less once a body part reaches its damage limit.
        assert!((0.0..=9.0).contains(&dealt), "a punch deals 1-9, dealt {dealt}");
    }
    // The attacker sees every attempt. 90% land; with this seed, well within 35..=50.
    let messages = ids(&app, alice);
    // A hard enough one also says what it did (`combat.md`).
    let attempts: Vec<&String> = messages
        .iter()
        .filter(|m| ["punched", "punched-face", "punch-missed"].contains(&m.as_str()))
        .collect();
    assert_eq!(attempts.len(), 50, "{messages:?}");
    let landed = attempts.iter().filter(|m| **m != "punch-missed").count();
    assert!((35..=50).contains(&landed), "landed {landed}/50");
}

#[test]
fn below_zero_unconscious_below_minus_hundred_dead() {
    let mut app = setup();
    let bob = spawn_player_mob(app.world_mut(), TilePos::new(6, 4, 0));
    let listener = spawn_player_mob(app.world_mut(), TilePos::new(7, 4, 0));

    let mut body = app.world_mut().get_mut::<Body>(bob).unwrap();
    body.take_damage(Zone::Chest, 101.0, 0.0);
    tick(&mut app, 1);
    assert!(health(&app, bob) < 0.0);
    assert_eq!(stat(&app, bob), MobStat::Unconscious);
    assert_eq!(app.world().get::<Appearance>(bob).unwrap().state, "unconscious");

    // Out cold: no moving, no talking, no hearing.
    app.world_mut().get_mut::<MoveIntent>(bob).unwrap().dir = Some(Dir::East);
    app.world_mut()
        .get_mut::<PendingSpeech>(bob)
        .unwrap()
        .0
        .push("help".into());
    let start = app.world().get::<GridPos>(bob).unwrap().0;
    app.world_mut()
        .get_mut::<PendingSpeech>(listener)
        .unwrap()
        .0
        .push("wake up".into());
    tick(&mut app, 20);
    assert_eq!(app.world().get::<GridPos>(bob).unwrap().0, start);
    assert!(ids(&app, bob).is_empty());
    assert_eq!(ids(&app, listener), ["say"], "only its own line");

    let mut body = app.world_mut().get_mut::<Body>(bob).unwrap();
    body.take_damage(Zone::Head, 125.0, 0.0);
    tick(&mut app, 1);
    assert_eq!(stat(&app, bob), MobStat::Dead);
    assert_eq!(app.world().get::<Appearance>(bob).unwrap().state, "dead");
}

#[test]
fn a_crowbar_beating_eventually_knocks_out() {
    let mut app = setup();
    let alice = spawn_player_mob(app.world_mut(), TilePos::new(5, 4, 0));
    let bob = spawn_player_mob(app.world_mut(), TilePos::new(6, 4, 0));
    let crowbar = find(&mut app, "crowbar");
    give(&mut app, alice, crowbar);
    let mut hits = 0;
    while stat(&app, bob) == MobStat::Conscious {
        act(&mut app, alice, Action::UseOn(bob));
        tick(&mut app, 1);
        hits += 1;
        assert!(hits < 100);
    }
    // 5 damage a hit: 21 hits take a human from 100 to below 0.
    assert_eq!(hits, 21);
    assert_eq!(stat(&app, bob), MobStat::Unconscious);
}

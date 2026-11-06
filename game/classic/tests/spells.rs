//! Spells. Spec: docs/reference/spells.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::combat::Muted;
use rs13_classic::doors::Airlock;
use rs13_classic::explosions::PendingBlasts;
use rs13_classic::flashes::Weakened;
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::speech::PendingSpeech;
use rs13_classic::spells::{
    self, Fireball, Forcefield, MIME_SPELL, Missile, SpellOp, SpellPanelOut, Spellcaster, teleport_areas,
};
use rs13_world::{Dense, Facing, GridPos, HumanLook, MapLayer, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    area_legend: { 'r': "room", 'b': "bridge" },
    area_rows: [
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
        "rrrrrrrrrrrrrrbbbbbb",
    ],
    rows: [
        "WWWWWWWWWWWWWWWWWWWW",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W.............W....W",
        "WWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn person(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(x, y, 0));
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn wizard(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = person(app, x, y);
    for proto in ["wizard-robe", "sandals", "wizard-hat"] {
        let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), TilePos::new(x, y, 0)).unwrap();
        rs13_classic::jobs::wear(app.world_mut(), mob, item);
    }
    mob
}

fn cast(app: &mut App, mob: Entity, line: u8, area: Option<u16>) {
    spells::command(
        app.world_mut(),
        mob,
        SpellOp::Cast {
            spell: line,
            target: None,
            area,
        },
    );
}

fn heard(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn count<C: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&C>().iter(world).count()
}

#[test]
fn only_real_spells_are_given_once_and_the_window_opens_for_casters() {
    let mut app = setup();
    let mob = person(&mut app, 2, 2);
    spells::command(app.world_mut(), mob, SpellOp::Open);
    assert!(
        app.world().get::<SpellPanelOut>(mob).is_none(),
        "no spells, no window"
    );
    assert!(!spells::give(app.world_mut(), mob, "summon-cheese"));
    assert!(spells::give(app.world_mut(), mob, "knock"));
    assert!(!spells::give(app.world_mut(), mob, "knock"));
    spells::command(app.world_mut(), mob, SpellOp::Open);
    tick(&mut app, 1);
    let view = app.world().get::<SpellPanelOut>(mob).unwrap().0.clone().unwrap();
    assert_eq!(view.spells.len(), 1);
    assert_eq!(view.spells[0].id, "knock");
    assert_eq!(view.spells[0].now, view.spells[0].max, "starts ready");
    assert!(view.areas.contains(&"bridge".to_owned()));
}

#[test]
fn a_magic_missile_needs_the_robes_and_homes_in_on_everyone_in_sight() {
    let mut app = setup();
    let plain = person(&mut app, 2, 2);
    spells::give(app.world_mut(), plain, "magic-missile");
    let victim = person(&mut app, 8, 2);
    cast(&mut app, plain, 0, None);
    assert!(heard(&app, plain, "spell-no-robe"));
    assert_eq!(count::<Missile>(&mut app), 0);

    let caster = wizard(&mut app, 2, 4);
    spells::give(app.world_mut(), caster, "magic-missile");
    cast(&mut app, caster, 0, None);
    // One each for the two others in sight.
    assert_eq!(count::<Missile>(&mut app), 2);
    let words = app.world().get::<PendingSpeech>(caster).unwrap().0.clone();
    assert!(
        words.iter().any(|w| w.replace('`', " ") == "FORTI GY AMA"),
        "{words:?}"
    );
    cast(&mut app, caster, 0, None);
    assert!(heard(&app, caster, "spell-recharging"));
    assert_eq!(count::<Missile>(&mut app), 2);

    // A step every half second, six tiles away.
    tick(&mut app, 80);
    assert_eq!(count::<Missile>(&mut app), 0);
    let burned: f32 = Zone::ALL
        .iter()
        .map(|z| app.world().get::<Body>(victim).unwrap().organ(*z).burn)
        .sum();
    assert_eq!(burned, 10.0);
    assert!(app.world().get::<Weakened>(victim).is_some());
}

#[test]
fn with_no_one_in_sight_the_spell_isnt_spent() {
    let mut app = setup();
    let caster = wizard(&mut app, 2, 2);
    spells::give(app.world_mut(), caster, "magic-missile");
    cast(&mut app, caster, 0, None);
    assert_eq!(
        app.world().get::<Spellcaster>(caster).unwrap().known[0].ready_at,
        0.0
    );
}

#[test]
fn someone_not_a_person_casts_for_nothing_and_a_muted_person_cant() {
    let mut app = setup();
    let monkey = spawn_player_mob(app.world_mut(), TilePos::new(2, 2, 0));
    spells::give(app.world_mut(), monkey, "forcewall");
    cast(&mut app, monkey, 0, None);
    cast(&mut app, monkey, 0, None);
    assert_eq!(count::<Forcefield>(&mut app), 2, "Luna never charges them");

    let muted = person(&mut app, 5, 5);
    app.world_mut().entity_mut(muted).insert(Muted);
    spells::give(app.world_mut(), muted, "forcewall");
    cast(&mut app, muted, 0, None);
    assert!(heard(&app, muted, "spell-cant-speak"));
    assert_eq!(count::<Forcefield>(&mut app), 2);
}

#[test]
fn a_forcewall_stands_where_cast_for_thirty_seconds_and_the_mimes_cant_be_seen() {
    let mut app = setup();
    let caster = person(&mut app, 3, 3);
    spells::give(app.world_mut(), caster, "forcewall");
    cast(&mut app, caster, 0, None);
    let world = app.world_mut();
    let mut walls =
        world.query_filtered::<(&GridPos, Has<rs13_world::Appearance>), (With<Forcefield>, With<Dense>)>();
    let (at, seen) = walls.single(world).unwrap();
    assert_eq!(at.0, TilePos::new(3, 3, 0));
    assert!(seen);

    let mime = person(&mut app, 6, 6);
    spells::learn_for_job(app.world_mut(), mime, "mime");
    assert_eq!(
        app.world().get::<Spellcaster>(mime).unwrap().known[0].id,
        MIME_SPELL
    );
    cast(&mut app, mime, 0, None);
    let world = app.world_mut();
    let mut walls = world.query_filtered::<(&GridPos, Has<rs13_world::Appearance>), With<Forcefield>>();
    assert!(
        walls
            .iter(world)
            .any(|(p, seen)| p.0 == TilePos::new(6, 6, 0) && !seen)
    );

    tick(&mut app, 610);
    assert_eq!(count::<Forcefield>(&mut app), 0);
}

#[test]
fn teleport_lands_in_the_chosen_area_and_names_it() {
    let mut app = setup();
    let caster = wizard(&mut app, 2, 2);
    spells::give(app.world_mut(), caster, "teleport");
    let areas = teleport_areas(app.world_mut());
    let bridge = areas.iter().position(|a| a == "bridge").unwrap() as u16;
    cast(&mut app, caster, 0, None);
    assert_eq!(
        app.world().get::<GridPos>(caster).unwrap().0,
        TilePos::new(2, 2, 0),
        "needs a place"
    );
    cast(&mut app, caster, 0, Some(bridge));
    let at = app.world().get::<GridPos>(caster).unwrap().0;
    let world = app.world_mut();
    let area = world
        .query::<&MapLayer>()
        .iter(world)
        .next()
        .unwrap()
        .0
        .area(at)
        .map(str::to_owned);
    assert_eq!(area.as_deref(), Some("bridge"));
    let words = app.world().get::<PendingSpeech>(caster).unwrap().0.clone();
    assert!(
        words.iter().any(|w| w.replace('`', " ") == "SCYAR NILA BRIDGE"),
        "{words:?}"
    );
}

#[test]
fn blink_jumps_up_to_six_tiles_and_keeps_off_the_edge() {
    let mut app = setup();
    let caster = wizard(&mut app, 9, 7);
    spells::give(app.world_mut(), caster, "blink");
    for _ in 0..10 {
        let from = app.world().get::<GridPos>(caster).unwrap().0;
        cast(&mut app, caster, 0, None);
        let to = app.world().get::<GridPos>(caster).unwrap().0;
        assert!((to.x - from.x).abs().max((to.y - from.y).abs()) <= 6);
        assert!(to.x >= 6 && to.y >= 6 && to.x < 14 && to.y < 8, "{to:?}");
        tick(&mut app, 5);
    }
}

#[test]
fn a_fireball_flies_the_way_its_caster_faces_and_bursts_at_the_wall() {
    let mut app = setup();
    let caster = person(&mut app, 2, 1);
    app.world_mut().entity_mut(caster).insert(Facing(2));
    spells::give(app.world_mut(), caster, "fireball");
    cast(&mut app, caster, 0, None);
    assert_eq!(count::<Fireball>(&mut app), 1);
    let mut burst = None;
    for _ in 0..60 {
        tick(&mut app, 1);
        if let Some(blast) = app.world().resource::<PendingBlasts>().0.first() {
            burst = Some((blast.at, blast.light));
            break;
        }
    }
    // The wall at x 14 on the bottom row stops it.
    assert_eq!(burst, Some((TilePos::new(13, 1, 0), 2)));
}

#[test]
fn knock_unbolts_doors_in_reach() {
    let mut app = setup();
    let near = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("airlock"),
        TilePos::new(5, 5, 0),
    )
    .unwrap();
    let far = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("airlock"),
        TilePos::new(12, 5, 0),
    )
    .unwrap();
    for door in [near, far] {
        app.world_mut().get_mut::<Airlock>(door).unwrap().bolted = true;
    }
    let caster = person(&mut app, 3, 5);
    spells::give(app.world_mut(), caster, "knock");
    cast(&mut app, caster, 0, None);
    assert!(!app.world().get::<Airlock>(near).unwrap().bolted);
    assert!(app.world().get::<Airlock>(far).unwrap().bolted);
}

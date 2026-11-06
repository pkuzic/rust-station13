//! The wizard's things: the spellbook, the scrolls, the magic staff and the scrying orb.
//! Spec: docs/reference/spells.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cloning::Arrivals;
use rs13_classic::doors::Airlock;
use rs13_classic::filing::{PendingPicks, PickListOut};
use rs13_classic::ghosts::{GhostOf, LeftBody};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::spells::{self, Forcefield, teleport_areas};
use rs13_classic::traitor::Traitor;
use rs13_classic::wizard_items::{BOOK_SPELLS, Choice, MayReturn, SUMMONED_GUNS, SpellbookOut, choices};
use rs13_world::{Appearance, Ghost, GridPos, HumanLook, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    area_legend: { 'r': "room", 'b': "bridge", 'w': "wizard-station" },
    area_rows: [
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
        "rrrrrrrrrrrrbbbbwwww",
    ],
    rows: [
        "WWWWWWWWWWWWWWWWWWWW",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "W..................W",
        "WWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
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

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

/// Put a new `proto` in the mob's active hand.
fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let at = app.world().get::<GridPos>(mob).unwrap().0;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), at).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(item));
    item
}

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn line_of(choice: Choice) -> usize {
    choices().iter().position(|c| *c == choice).unwrap()
}

fn uses(app: &App, mob: Entity) -> u8 {
    app.world()
        .get::<SpellbookOut>(mob)
        .and_then(|o| o.0.as_ref())
        .expect("the book is open")
        .uses
}

fn things_at(app: &mut App, at: TilePos) -> Vec<String> {
    let world = app.world_mut();
    let mut q = world.query::<(&GridPos, &Appearance)>();
    q.iter(world)
        .filter(|(p, _)| p.0 == at)
        .map(|(_, a)| a.sprite.clone())
        .collect()
}

#[test]
fn the_book_teaches_each_spell_once_and_sells_artefacts_from_its_six() {
    let mut app = setup();
    let wizard = person(&mut app, 3, 3);
    let book = hold(&mut app, wizard, "spellbook");
    act(&mut app, wizard, Action::UseOn(book));
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<SpellbookOut>(wizard)
        .unwrap()
        .0
        .clone()
        .unwrap();
    assert_eq!(view.uses, 6);
    assert_eq!(
        view.spells.len(),
        BOOK_SPELLS.len() + 1,
        "the spells, then Summon Guns"
    );
    assert!(view.spells.contains(&("magic-missile".to_owned(), 15)));
    assert_eq!(view.spells.last(), Some(&("summon-guns".to_owned(), 0)));
    assert_eq!(view.artefacts, ["armor", "scrying"]);

    pick(&mut app, wizard, book, line_of(Choice::Spell("fireball")));
    assert!(spells::knows(app.world(), wizard, "fireball"));
    assert!(told(&app, wizard, "book-learned"));
    assert_eq!(uses(&app, wizard), 5);
    // Again: they know it, and it costs nothing.
    pick(&mut app, wizard, book, line_of(Choice::Spell("fireball")));
    assert!(told(&app, wizard, "book-known"));
    assert_eq!(uses(&app, wizard), 5);

    pick(&mut app, wizard, book, line_of(Choice::Armor));
    tick(&mut app, 1);
    let here = things_at(&mut app, TilePos::new(3, 3, 0));
    for id in [
        "sandals",
        "purple-gloves",
        "wizard-hardsuit",
        "wizard-hardsuit-helmet",
    ] {
        assert!(here.contains(&id.to_owned()), "{id} at their feet: {here:?}");
    }
    assert_eq!(uses(&app, wizard), 4);

    // Re-memorizing only works in the wizard's sanctuary.
    pick(&mut app, wizard, book, line_of(Choice::Rememorize));
    assert!(told(&app, wizard, "book-rememorize-elsewhere"));
    assert!(spells::knows(app.world(), wizard, "fireball"));
    app.world_mut()
        .entity_mut(wizard)
        .insert(GridPos(TilePos::new(17, 3, 0)));
    // Carried along: the book is in their hand.
    pick(&mut app, wizard, book, line_of(Choice::Rememorize));
    assert!(told(&app, wizard, "book-rememorized"));
    assert!(!spells::knows(app.world(), wizard, "fireball"));
    // The armour is bought for good: five left, not six.
    assert_eq!(uses(&app, wizard), 5);
}

#[test]
fn summon_guns_arms_everyone_else_too() {
    let mut app = setup();
    let wizard = person(&mut app, 3, 3);
    let crew = person(&mut app, 8, 4);
    let book = hold(&mut app, wizard, "spellbook");
    act(&mut app, wizard, Action::UseOn(book));
    pick(&mut app, wizard, book, line_of(Choice::SummonGuns));
    assert!(told(&app, wizard, "book-summoned-guns"));
    tick(&mut app, 1);
    for (mob, at) in [(wizard, TilePos::new(3, 3, 0)), (crew, TilePos::new(8, 4, 0))] {
        let here = things_at(&mut app, at);
        assert!(
            here.iter().any(|s| SUMMONED_GUNS.contains(&s.as_str())),
            "a gun by {mob:?}: {here:?}"
        );
        // A survivor's only objective is to live.
        if let Some(traitor) = app.world().get::<Traitor>(mob) {
            assert_eq!(traitor.objectives.len(), 1);
        }
    }
}

#[test]
fn a_magic_scroll_teaches_its_spell_once() {
    let mut app = setup();
    let reader = person(&mut app, 3, 3);
    let scroll = hold(&mut app, reader, "magic-scroll-blink");
    act(&mut app, reader, Action::UseOn(scroll));
    assert!(spells::knows(app.world(), reader, "blink"));
    assert!(told(&app, reader, "scroll-learned"));

    let other = person(&mut app, 4, 3);
    act(&mut app, reader, Action::Drop);
    act(&mut app, other, Action::UseOn(scroll));
    act(&mut app, other, Action::UseOn(scroll));
    assert!(!spells::knows(app.world(), other, "blink"), "it's been read");
}

#[test]
fn the_teleportation_scroll_jumps_four_times_to_the_place_picked() {
    let mut app = setup();
    let wizard = person(&mut app, 3, 3);
    let scroll = hold(&mut app, wizard, "teleportation-scroll");
    act(&mut app, wizard, Action::UseOn(scroll));
    tick(&mut app, 1);
    let rows = app
        .world()
        .get::<PickListOut>(wizard)
        .unwrap()
        .0
        .clone()
        .unwrap()
        .rows;
    let areas = teleport_areas(app.world_mut());
    assert_eq!(rows, areas);
    assert!(!rows.iter().any(|r| r.contains("wizard")));
    let bridge = areas.iter().position(|a| a == "bridge").unwrap();
    let room = areas.iter().position(|a| a == "room").unwrap();

    for (i, line) in [bridge, room, bridge, room].into_iter().enumerate() {
        pick(&mut app, wizard, scroll, line);
        let at = app.world().get::<GridPos>(wizard).unwrap().0;
        let want = if line == bridge { 12..=15 } else { 1..=11 };
        assert!(want.contains(&at.x), "jump {i} to {at:?}");
    }
    let before = app.world().get::<GridPos>(wizard).unwrap().0;
    pick(&mut app, wizard, scroll, bridge);
    assert_eq!(
        app.world().get::<GridPos>(wizard).unwrap().0,
        before,
        "no jumps left"
    );
}

#[test]
fn the_magic_staff_walls_off_floor_and_knocks_doors_from_afar() {
    let mut app = setup();
    let wizard = person(&mut app, 2, 3);
    let door = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("airlock"),
        TilePos::new(9, 5, 0),
    )
    .unwrap();
    app.world_mut().get_mut::<Airlock>(door).unwrap().bolted = true;
    hold(&mut app, wizard, "magic-staff");

    act(&mut app, wizard, Action::UseOnTile(TilePos::new(7, 3, 0)));
    let world = app.world_mut();
    let mut walls = world.query_filtered::<&GridPos, With<Forcefield>>();
    assert_eq!(walls.single(world).unwrap().0, TilePos::new(7, 3, 0));
    tick(&mut app, 110);
    let world = app.world_mut();
    let mut walls = world.query_filtered::<&GridPos, With<Forcefield>>();
    assert_eq!(walls.iter(world).count(), 0, "gone after five seconds");

    act(&mut app, wizard, Action::UseOn(door));
    assert!(!app.world().get::<Airlock>(door).unwrap().bolted);
}

#[test]
fn the_scrying_orb_lets_you_out_as_a_ghost_and_back_in() {
    let mut app = setup();
    let wizard = person(&mut app, 3, 3);
    let orb = hold(&mut app, wizard, "scrying-orb");
    act(&mut app, wizard, Action::UseOn(orb));
    assert!(told(&app, wizard, "orb-see-everything"));
    let ghost = app.world().get::<LeftBody>(wizard).expect("out of the body").0;
    assert!(app.world().get::<Ghost>(ghost).is_some());
    assert!(app.world().get::<MayReturn>(ghost).is_some());
    assert_eq!(app.world().get::<GhostOf>(ghost).unwrap().0, wizard);

    act(&mut app, ghost, Action::UseOn(wizard));
    assert!(app.world().get::<LeftBody>(wizard).is_none());
    assert!(
        app.world().resource::<Arrivals>().0.contains(&(ghost, wizard)),
        "the player goes back in"
    );
}

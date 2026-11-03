//! Cards held and worn count alike, for everything that checks access. Spec: docs/reference/access.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::closets::ClosetLock;
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::windoors::{Swing, Windoor};
use rs13_world::{Dir, GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const HERE: TilePos = TilePos::new(1, 1, 0);
const NEXT: TilePos = TilePos::new(2, 1, 0);

/// How someone shows a card.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Showing {
    Nothing,
    Held,
    Worn,
    /// A useless card in hand, and the right one worn.
    WrongHeldRightWorn,
    /// The wrong card in both.
    WrongBoth,
}
use Showing::*;

fn card(app: &mut App, access: &str) -> Entity {
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), HERE).unwrap();
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec![access.to_owned()],
    });
    card
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn wear(app: &mut App, mob: Entity, card: Entity) {
    app.world_mut()
        .entity_mut(card)
        .remove::<GridPos>()
        .insert(WornBy(mob));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(Slot::Id, card);
}

/// Someone next to a thing with this access, showing a card as asked.
fn setup(proto: &str, access: &str, showing: Showing) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let mut spec = ObjectSpec::new(proto);
    spec.access = vec![access.to_owned()];
    let thing = spawn_object(app.world_mut(), &spec, NEXT).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    let right = card(&mut app, access);
    let wrong = card(&mut app, "kitchen");
    match showing {
        Nothing => {}
        Held => click(&mut app, mob, right),
        Worn => wear(&mut app, mob, right),
        WrongHeldRightWorn => {
            wear(&mut app, mob, right);
            click(&mut app, mob, wrong);
        }
        WrongBoth => {
            wear(&mut app, mob, wrong);
            let other = card(&mut app, "bar");
            click(&mut app, mob, other);
        }
    }
    (app, mob, thing)
}

const CASES: [(Showing, bool); 5] = [
    (Nothing, false),
    (Held, true),
    (Worn, true),
    (WrongHeldRightWorn, true),
    (WrongBoth, false),
];

fn door_opened(app: &App, door: Entity) -> bool {
    !matches!(app.world().get::<Airlock>(door).unwrap().state, DoorState::Closed)
}

#[test]
fn clicking_an_airlock() {
    for (showing, opens) in CASES {
        let (mut app, mob, door) = setup("airlock", "engine", showing);
        click(&mut app, mob, door);
        assert_eq!(door_opened(&app, door), opens, "{showing:?}");
    }
}

#[test]
fn walking_into_an_airlock() {
    for (showing, opens) in CASES {
        let (mut app, mob, door) = setup("airlock", "engine", showing);
        app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
        for _ in 0..4 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert_eq!(door_opened(&app, door), opens, "{showing:?}");
    }
}

#[test]
fn clicking_a_window_door() {
    for (showing, opens) in CASES {
        let (mut app, mob, door) = setup("windoor", "engine", showing);
        click(&mut app, mob, door);
        let swing = app.world().get::<Windoor>(door).unwrap().swing;
        assert_eq!(swing != Swing::Closed, opens, "{showing:?}");
    }
}

#[test]
fn a_locker() {
    for (showing, works) in CASES {
        let (mut app, mob, locker) = setup("locker-scientist", "tox_storage", showing);
        let before = app.world().get::<ClosetLock>(locker).unwrap().locked;
        // A card in hand or none, the lock is what a click on a locked locker works.
        click(&mut app, mob, locker);
        let after = app.world().get::<ClosetLock>(locker).unwrap().locked;
        assert_eq!(after != before, works, "{showing:?}");
    }
}

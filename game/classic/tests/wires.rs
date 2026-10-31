//! Airlock panels and wires in the engineering chamber. Spec: docs/reference/wires.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::equipment::{EquipAction, PendingEquipment};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Body;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::rng::GameRng;
use rs13_classic::wires::{PendingWires, WireFn, WireLayout, WireOp, WirePanelOut, Wiring};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, Dir, GridPos, Slot, TileMap, TilePos};

const TICKS_PER_SECOND: usize = 20;
/// The closet door: needs `engine_equip`.
const DOOR: TilePos = TilePos::new(16, 6, 0);
const BESIDE: TilePos = TilePos::new(15, 6, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    let door = {
        let mut q = app.world_mut().query::<(Entity, &GridPos, &Airlock)>();
        q.iter(app.world()).find(|(_, p, _)| p.0 == DOOR).unwrap().0
    };
    ticks(&mut app, 1);
    // Rolls made while the map starts up (lights, for one) don't change the shocks here.
    *app.world_mut().resource_mut::<GameRng>() = GameRng::default();
    (app, mob, door)
}

fn ticks(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app
        .world_mut()
        .query::<(Entity, &Appearance, Option<&GridPos>, Option<&HeldBy>)>();
    q.iter(app.world())
        .find(|(_, a, p, h)| a.sprite == sprite && (p.is_some() || h.is_some()))
        .unwrap_or_else(|| panic!("no {sprite}"))
        .0
}

/// Put `sprite` in the active hand, dropping whatever was there.
fn hold(app: &mut App, mob: Entity, sprite: &str) {
    let item = find(app, sprite);
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    let old = hands.slot_mut(active).replace(item);
    if let Some(old) = old.filter(|o| *o != item) {
        app.world_mut()
            .entity_mut(old)
            .remove::<HeldBy>()
            .insert(GridPos(BESIDE));
    }
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
}

fn empty_hand(app: &mut App, mob: Entity) {
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    if let Some(old) = hands.slot_mut(active).take() {
        app.world_mut()
            .entity_mut(old)
            .remove::<HeldBy>()
            .insert(GridPos(BESIDE));
    }
}

fn use_on(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
}

fn wire(app: &mut App, mob: Entity, op: WireOp) {
    app.world_mut().get_mut::<PendingWires>(mob).unwrap().0.push(op);
    ticks(app, 1);
}

fn colour(app: &App, function: WireFn) -> usize {
    app.world().resource::<WireLayout>().colour_of(function)
}

fn door(app: &App, door: Entity) -> Airlock {
    app.world().get::<Airlock>(door).unwrap().clone()
}

fn last_message(app: &App, mob: Entity) -> String {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .map(|m| m.id.clone())
        .unwrap_or_default()
}

fn wear_insulated_gloves(app: &mut App, mob: Entity) {
    hold(app, mob, "insulated-gloves");
    app.world_mut()
        .get_mut::<PendingEquipment>(mob)
        .unwrap()
        .0
        .push(EquipAction::Equip(Slot::Gloves));
    ticks(app, 1);
}

/// Screwdriver the panel open, then look inside.
fn open_panel(app: &mut App, mob: Entity, airlock: Entity) {
    hold(app, mob, "screwdriver");
    use_on(app, mob, airlock);
    assert!(door(app, airlock).panel_open);
    assert_eq!(last_message(app, mob), "door-panel-opened");
    hold(app, mob, "wirecutters");
    use_on(app, mob, airlock);
    ticks(app, 1);
    let view = app.world().get::<WirePanelOut>(mob).unwrap().0.clone();
    let view = view.expect("the wire panel is open");
    assert_eq!(view.wires.len(), 11);
}

#[test]
fn an_open_panel_shows_wires_instead_of_opening_the_door() {
    let (mut app, mob, airlock) = setup();
    open_panel(&mut app, mob, airlock);
    assert_eq!(door(&app, airlock).state, DoorState::Closed);
    assert!(
        app.world()
            .get::<Appearance>(airlock)
            .unwrap()
            .state
            .ends_with("+panel")
    );

    // Walking into it does nothing either (Luna `Bumped`).
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
    ticks(&mut app, 2 * TICKS_PER_SECOND);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    assert_eq!(door(&app, airlock).state, DoorState::Closed);

    // Stepping away closes the panel view.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(12, 6, 0);
    ticks(&mut app, 2);
    assert!(app.world().get::<WirePanelOut>(mob).unwrap().0.is_none());
}

#[test]
fn cutting_power_lets_the_door_be_crowbarred() {
    let (mut app, mob, airlock) = setup();
    wear_insulated_gloves(&mut app, mob);
    open_panel(&mut app, mob, airlock);
    let main = colour(&app, WireFn::MainPower1);
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: main,
        },
    );
    assert!(!door(&app, airlock).has_power(), "main power lost");
    // Backup takes over after 10 s.
    ticks(&mut app, 11 * TICKS_PER_SECOND);
    assert!(door(&app, airlock).has_power(), "backup power");

    let backup = colour(&app, WireFn::BackupPower2);
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: backup,
        },
    );
    assert!(!door(&app, airlock).has_power());
    ticks(&mut app, 70 * TICKS_PER_SECOND);
    assert!(!door(&app, airlock).has_power(), "stays dead while both are cut");
    assert_eq!(
        app.world().get::<Body>(mob).unwrap().health(),
        100.0,
        "insulated gloves"
    );

    hold(&mut app, mob, "crowbar");
    use_on(&mut app, mob, airlock);
    assert!(matches!(door(&app, airlock).state, DoorState::Opening { .. }));

    // Mending both restores main power at once.
    hold(&mut app, mob, "wirecutters");
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: main,
        },
    );
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: backup,
        },
    );
    assert!(door(&app, airlock).has_power());
}

#[test]
fn bare_hands_on_power_wires_get_shocked() {
    let (mut app, mob, airlock) = setup();
    open_panel(&mut app, mob, airlock);
    let main = colour(&app, WireFn::MainPower2);
    for _ in 0..8 {
        wire(
            &mut app,
            mob,
            WireOp::CutOrMend {
                door: airlock,
                colour: main,
            },
        );
    }
    assert!(app.world().get::<Body>(mob).unwrap().health() < 100.0);
    assert!(
        app.world()
            .get::<Outbox>(mob)
            .unwrap()
            .0
            .iter()
            .any(|m| m.id == "shocked")
    );
}

#[test]
fn tools_matter() {
    let (mut app, mob, airlock) = setup();
    open_panel(&mut app, mob, airlock);
    let bolts = colour(&app, WireFn::Bolts);
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: bolts,
        },
    );
    assert_eq!(last_message(&app, mob), "wire-need-multitool");
    empty_hand(&mut app, mob);
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: bolts,
        },
    );
    assert_eq!(last_message(&app, mob), "wire-need-cutters");

    // Pulsing the bolt wire drops the bolts, pulsing again raises them.
    hold(&mut app, mob, "multitool");
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: bolts,
        },
    );
    assert!(door(&app, airlock).bolted);
    assert!(
        app.world()
            .get::<Appearance>(airlock)
            .unwrap()
            .state
            .starts_with("locked")
    );
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: bolts,
        },
    );
    assert!(!door(&app, airlock).bolted);

    // A cut wire cannot be pulsed.
    hold(&mut app, mob, "wirecutters");
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: bolts,
        },
    );
    assert!(door(&app, airlock).bolted, "cutting drops the bolts");
    hold(&mut app, mob, "multitool");
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: bolts,
        },
    );
    assert_eq!(last_message(&app, mob), "wire-pulse-cut");
}

#[test]
fn a_cut_id_scanner_lets_the_open_wire_through() {
    let (mut app, mob, airlock) = setup();
    open_panel(&mut app, mob, airlock);
    let open = colour(&app, WireFn::OpenDoor);
    hold(&mut app, mob, "multitool");
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: open,
        },
    );
    assert_eq!(
        door(&app, airlock).state,
        DoorState::Closed,
        "the door wants an ID"
    );

    hold(&mut app, mob, "wirecutters");
    let id = colour(&app, WireFn::IdScan);
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: id,
        },
    );
    hold(&mut app, mob, "multitool");
    wire(
        &mut app,
        mob,
        WireOp::Pulse {
            door: airlock,
            colour: open,
        },
    );
    assert!(matches!(door(&app, airlock).state, DoorState::Opening { .. }));
}

#[test]
fn a_cut_electrify_wire_bites_whoever_touches_the_door() {
    let (mut app, mob, airlock) = setup();
    open_panel(&mut app, mob, airlock);
    let shock = colour(&app, WireFn::Electrify);
    wire(
        &mut app,
        mob,
        WireOp::CutOrMend {
            door: airlock,
            colour: shock,
        },
    );
    assert!(app.world().get::<Wiring>(airlock).unwrap().is_electrified());
    // Close the panel: the door is now a trap.
    hold(&mut app, mob, "screwdriver");
    use_on(&mut app, mob, airlock);
    ticks(&mut app, 1);
    assert!(
        app.world().get::<Body>(mob).unwrap().health() < 100.0,
        "the screwdriver touched it"
    );
}

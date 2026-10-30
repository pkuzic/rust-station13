//! Air alarms and emergency seals in the engineering chamber.
//! Spec: docs/reference/air-alarm.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::alarms::{AirAlarm, AlarmCommand, AlarmPanelOut, AreaSeals, PendingAlarmOps};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::Vent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::wires::{WireFn, WireLayout, Wiring};
use rs13_classic::{ClassicPlugin, maps};
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, Dir, GridPos, TileMap, TilePos};

const TICKS_PER_SECOND: usize = 20;
const INNER_DOOR: TilePos = TilePos::new(16, 6, 0);
const OUTER_DOOR: TilePos = TilePos::new(20, 6, 0);
const CLOSET: TilePos = TilePos::new(18, 8, 0);
const ROOM: TilePos = TilePos::new(8, 8, 0);
/// The alarms on the west wall of the room and the east wall of the closet.
const ROOM_ALARM: TilePos = TilePos::new(1, 5, 0);
const CLOSET_ALARM: TilePos = TilePos::new(20, 3, 0);
const CLOSET_VENT: TilePos = TilePos::new(18, 10, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn ticks(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn seconds(app: &mut App, s: usize) {
    ticks(app, s * TICKS_PER_SECOND);
}

fn at<C: Component>(app: &mut App, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query_filtered::<(Entity, &GridPos), With<C>>();
    q.iter(app.world()).find(|(_, p)| p.0 == pos).unwrap().0
}

fn door(app: &mut App, pos: TilePos) -> Airlock {
    let entity = at::<Airlock>(app, pos);
    app.world().get::<Airlock>(entity).unwrap().clone()
}

/// Jam a door open (no autoclose), as if someone just walked through.
fn hold_open(app: &mut App, pos: TilePos) {
    let entity = at::<Airlock>(app, pos);
    app.world_mut().get_mut::<Airlock>(entity).unwrap().state = DoorState::Open { autoclose_at: None };
}

/// Cut a door's main and backup power wires.
fn kill_door(app: &mut App, pos: TilePos) {
    let entity = at::<Airlock>(app, pos);
    let layout = app.world().resource::<WireLayout>().clone();
    let mut door = app.world().get::<Airlock>(entity).unwrap().clone();
    let mut wiring = app.world().get::<Wiring>(entity).unwrap().clone();
    for function in [WireFn::MainPower1, WireFn::BackupPower1] {
        wiring.cut(&layout, layout.colour_of(function), &mut door);
    }
    app.world_mut().entity_mut(entity).insert((door, wiring));
}

fn pressure(app: &App, pos: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(pos)
}

fn sealed(app: &App, area: &str) -> bool {
    app.world().resource::<AreaSeals>().0.contains(area)
}

fn alarm_look(app: &mut App, pos: TilePos) -> String {
    let alarm = at::<AirAlarm>(app, pos);
    app.world().get::<Appearance>(alarm).unwrap().state.clone()
}

fn command(app: &mut App, mob: Entity, alarm: Entity, command: AlarmCommand) {
    app.world_mut()
        .get_mut::<PendingAlarmOps>(mob)
        .unwrap()
        .0
        .push((alarm, command));
    ticks(app, 2);
}

#[test]
fn a_breach_seals_the_closet_until_the_vents_refill_it() {
    let mut app = setup();
    seconds(&mut app, 2);
    assert_eq!(alarm_look(&mut app, CLOSET_ALARM), "alarm0");
    hold_open(&mut app, OUTER_DOOR);

    // Luna waits 5 s after the air turns dangerous; the alarm checks every 2 s.
    seconds(&mut app, 3);
    assert_eq!(alarm_look(&mut app, CLOSET_ALARM), "alarm1");
    assert!(!sealed(&app, "eng-storage"), "not yet");
    seconds(&mut app, 6);
    assert!(sealed(&app, "eng-storage"));
    assert!(!sealed(&app, "engineering"), "the room's air is fine");

    // The doors close and bolt once shut.
    seconds(&mut app, 1);
    let outer = door(&mut app, OUTER_DOOR);
    assert_eq!(outer.state, DoorState::Closed);
    assert!(outer.bolted && outer.air_locked);
    let inner = door(&mut app, INNER_DOOR);
    assert!(inner.bolted && inner.air_locked, "the closet door is sealed too");
    assert!((pressure(&app, ROOM) - ONE_ATMOSPHERE).abs() < 1e-6);

    // The vent refills the closet; once the air is no longer dangerous the seal lifts and
    // the bolts it dropped go up again.
    let mut lifted_at = None;
    for s in 1..=180 {
        seconds(&mut app, 1);
        if !sealed(&app, "eng-storage") {
            lifted_at = Some(s);
            break;
        }
    }
    let at = lifted_at.unwrap_or_else(|| panic!("still sealed at {:.1} kPa", pressure(&app, CLOSET)));
    assert!(
        pressure(&app, CLOSET) > 0.7 * ONE_ATMOSPHERE,
        "lifted after {at} s"
    );
    ticks(&mut app, 1);
    for pos in [OUTER_DOOR, INNER_DOOR] {
        let d = door(&mut app, pos);
        assert!(!d.bolted && !d.air_locked, "{pos:?} unbolted");
    }
}

#[test]
fn a_dead_door_is_not_sealed_but_the_rest_are() {
    // The milestone's story: cut the outer door's power, prise it open, and the alarm can
    // only save the room by sealing the inner door.
    let mut app = setup();
    kill_door(&mut app, OUTER_DOOR);
    hold_open(&mut app, OUTER_DOOR);
    seconds(&mut app, 12);
    assert!(sealed(&app, "eng-storage"));
    assert!(matches!(door(&mut app, OUTER_DOOR).state, DoorState::Open { .. }));
    let inner = door(&mut app, INNER_DOOR);
    assert!(inner.bolted && inner.air_locked);
    seconds(&mut app, 30);
    assert!(pressure(&app, CLOSET) < 1.0, "still open to space");
    assert!((pressure(&app, ROOM) - ONE_ATMOSPHERE).abs() < 1e-6);
}

#[test]
fn the_alarm_window_reads_the_room_and_its_buttons_work() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), ROOM_ALARM.step(Dir::East));
    let alarm = at::<AirAlarm>(&mut app, ROOM_ALARM);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(alarm));
    ticks(&mut app, 2);
    let view = app.world().get::<AlarmPanelOut>(mob).unwrap().0.clone();
    let view = view.expect("the window is open");
    assert_eq!(view.area, "engineering");
    assert_eq!(view.pressure_dkpa, 1013);
    assert!(!view.pressure_alert);
    let ids: Vec<&str> = view.readings.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        ["alarm-nitrogen", "alarm-oxygen", "alarm-co2", "alarm-temperature"]
    );
    assert_eq!(view.readings[1].value, 21);
    assert_eq!(view.readings[3].value, 20);
    assert!(view.controls);
    assert_eq!(view.panic_fill, Some(false));
    assert!(!view.sealed);

    // The emergency seal by hand bolts the closet door, which borders the room.
    command(&mut app, mob, alarm, AlarmCommand::Seal(true));
    assert!(sealed(&app, "engineering"));
    assert!(door(&mut app, INNER_DOOR).bolted);
    let view = app.world().get::<AlarmPanelOut>(mob).unwrap().0.clone();
    assert!(view.unwrap().sealed);
    command(&mut app, mob, alarm, AlarmCommand::Seal(false));
    assert!(!door(&mut app, INNER_DOOR).bolted);

    // From across the room the readouts stay but the buttons don't work.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 =
        ROOM_ALARM.step(Dir::East).step(Dir::East).step(Dir::East);
    ticks(&mut app, 1);
    let view = app.world().get::<AlarmPanelOut>(mob).unwrap().0.clone();
    assert!(!view.expect("still open within 3 tiles").controls);
    command(&mut app, mob, alarm, AlarmCommand::Seal(true));
    assert!(!sealed(&app, "engineering"), "out of reach");

    // Further away it closes.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = ROOM;
    ticks(&mut app, 1);
    assert!(app.world().get::<AlarmPanelOut>(mob).unwrap().0.is_none());
}

#[test]
fn wirecutters_disconnect_the_alarm() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), CLOSET_ALARM.step(Dir::West));
    let alarm = at::<AirAlarm>(&mut app, CLOSET_ALARM);
    let cutters = {
        let mut q = app.world_mut().query::<(Entity, &Appearance)>();
        q.iter(app.world())
            .find(|(_, a)| a.sprite == "wirecutters")
            .unwrap()
            .0
    };
    {
        let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
        let active = hands.active;
        *hands.slot_mut(active) = Some(cutters);
    }
    app.world_mut()
        .entity_mut(cutters)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(alarm));
    seconds(&mut app, 2);
    assert!(app.world().get::<AirAlarm>(alarm).unwrap().broken);
    assert_eq!(alarm_look(&mut app, CLOSET_ALARM), "alarmp");
    assert!(app.world().get::<AlarmPanelOut>(mob).unwrap().0.is_none());

    // A disconnected alarm never seals: the closet just vents.
    hold_open(&mut app, OUTER_DOOR);
    seconds(&mut app, 15);
    assert!(!sealed(&app, "eng-storage"));
    assert!(matches!(door(&mut app, OUTER_DOOR).state, DoorState::Open { .. }));
}

#[test]
fn panic_fill_refills_a_vented_closet_fast() {
    let mut app = setup();
    let alarm = at::<AirAlarm>(&mut app, CLOSET_ALARM);
    app.world_mut().get_mut::<AirAlarm>(alarm).unwrap().broken = true;
    hold_open(&mut app, OUTER_DOOR);
    seconds(&mut app, 30);
    assert!(pressure(&app, CLOSET) < 1.0, "vented");
    let outer = at::<Airlock>(&mut app, OUTER_DOOR);
    app.world_mut().get_mut::<Airlock>(outer).unwrap().state = DoorState::Closed;
    app.world_mut().get_mut::<AirAlarm>(alarm).unwrap().broken = false;

    let mob = spawn_player_mob(app.world_mut(), CLOSET_ALARM.step(Dir::West));
    command(&mut app, mob, alarm, AlarmCommand::PanicFill(true));
    let mut restored_at = None;
    for s in 1..=60 {
        seconds(&mut app, 1);
        if pressure(&app, CLOSET) > 95.0 {
            restored_at = Some(s);
            break;
        }
    }
    // The ordinary vent takes about two and a half minutes (docs/reference/pipes.md).
    let after = restored_at.unwrap_or_else(|| panic!("still {:.1} kPa after 60 s", pressure(&app, CLOSET)));
    assert!(after <= 30, "restored after {after} s");
    // It turns itself off once the room is back.
    seconds(&mut app, 4);
    let vent = at::<Vent>(&mut app, CLOSET_VENT);
    assert!(!app.world().get::<Vent>(vent).unwrap().panic);
}

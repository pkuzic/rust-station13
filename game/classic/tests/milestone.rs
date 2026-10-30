//! The first milestone's exit criterion (docs/SCOPE.md), played through as one story: two
//! players in the engineering chamber open the airlock, cut its power, cause a breach and
//! watch the pressure drop, in either language.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::alarms::{AirAlarm, AlarmPanelOut, AreaSeals};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::environment::Surroundings;
use rs13_classic::equipment::{EquipAction, PendingEquipment};
use rs13_classic::hands::HeldBy;
use rs13_classic::health::{Body, Vitals};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::wires::{PendingWires, WireFn, WireLayout, WireOp};
use rs13_classic::{ClassicPlugin, locales, maps};
use rs13_i18n::{Args, Lexicon, LocMsg, Localizer, langid};
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, GridPos, MobStat, Slot, TileMap, TilePos};

const TICKS_PER_SECOND: usize = 20;
const INNER_DOOR: TilePos = TilePos::new(16, 6, 0);
const OUTER_DOOR: TilePos = TilePos::new(20, 6, 0);
const ROOM_ALARM: TilePos = TilePos::new(1, 5, 0);
const CLOSET: TilePos = TilePos::new(18, 8, 0);
const ROOM: TilePos = TilePos::new(8, 8, 0);

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

/// Walking is covered elsewhere; here people just step where the story needs them.
fn go(app: &mut App, mob: Entity, pos: TilePos) {
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = pos;
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 1);
}

fn loose(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance, Option<&GridPos>)>();
    q.iter(app.world())
        .find(|(_, a, p)| a.sprite == sprite && p.is_some())
        .unwrap_or_else(|| panic!("no loose {sprite}"))
        .0
}

/// Pick an item up from the floor with the active hand, as a click would.
fn pick_up(app: &mut App, mob: Entity, item: Entity) {
    let pos = app.world().get::<GridPos>(item).unwrap().0;
    go(app, mob, pos);
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<HeldBy>(item).map(|h| h.0), Some(mob));
}

/// Put down whatever is in the active hand.
fn drop_held(app: &mut App, mob: Entity) {
    act(app, mob, Action::Drop);
}

fn wear(app: &mut App, mob: Entity, sprite: &str, slot: Slot) {
    let item = loose(app, sprite);
    pick_up(app, mob, item);
    app.world_mut()
        .get_mut::<PendingEquipment>(mob)
        .unwrap()
        .0
        .push(EquipAction::Equip(slot));
    ticks(app, 1);
}

fn door_at(app: &mut App, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Airlock)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == pos).unwrap().0
}

fn door(app: &mut App, pos: TilePos) -> Airlock {
    let entity = door_at(app, pos);
    app.world().get::<Airlock>(entity).unwrap().clone()
}

fn pressure_around(app: &App, mob: Entity) -> f64 {
    f64::from(app.world().get::<Surroundings>(mob).unwrap().pressure)
}

fn messages(app: &App, mob: Entity) -> Vec<LocMsg> {
    app.world().get::<Outbox>(mob).unwrap().0.clone()
}

fn localizers() -> [Localizer; 2] {
    [
        Localizer::new(
            langid!("en"),
            &[locales::EN_MESSAGES],
            Lexicon::from_ron(locales::EN_NOUNS).unwrap(),
        )
        .unwrap(),
        Localizer::new(
            langid!("ru"),
            &[locales::RU_MESSAGES],
            Lexicon::from_ron(locales::RU_NOUNS).unwrap(),
        )
        .unwrap(),
    ]
}

#[test]
fn two_players_open_the_airlock_cut_its_power_breach_it_and_watch_the_pressure_drop() {
    let mut app = setup();
    let alice = spawn_player_mob(app.world_mut(), ROOM);
    let bob = spawn_player_mob(app.world_mut(), ROOM);
    ticks(&mut app, 1);

    // Alice takes the engineer's ID card and opens the closet airlock with it.
    let card = {
        let mut q = app.world_mut().query::<(Entity, &IdCard)>();
        q.iter(app.world())
            .find(|(_, c)| c.access.iter().any(|a| a == "engine_equip"))
            .unwrap()
            .0
    };
    pick_up(&mut app, alice, card);
    go(&mut app, alice, INNER_DOOR.step(rs13_world::Dir::West));
    let inner = door_at(&mut app, INNER_DOOR);
    act(&mut app, alice, Action::UseOn(inner));
    assert!(matches!(
        door(&mut app, INNER_DOOR).state,
        DoorState::Opening { .. }
    ));

    // Bob goes in and gets into the EVA gear, with insulated gloves for the wires.
    wear(&mut app, bob, "space-suit", Slot::Suit);
    wear(&mut app, bob, "space-helmet", Slot::Head);
    wear(&mut app, bob, "breath-mask", Slot::Mask);
    wear(&mut app, bob, "emergency-oxygen", Slot::Belt);
    wear(&mut app, bob, "insulated-gloves", Slot::Gloves);
    app.world_mut()
        .get_mut::<PendingEquipment>(bob)
        .unwrap()
        .0
        .push(EquipAction::ToggleInternals);
    ticks(&mut app, 1);

    // He opens the outer airlock's panel and cuts its main and backup power.
    let outer = door_at(&mut app, OUTER_DOOR);
    let beside_outer = OUTER_DOOR
        .step(rs13_world::Dir::West)
        .step(rs13_world::Dir::South);
    let screwdriver = loose(&mut app, "screwdriver");
    pick_up(&mut app, bob, screwdriver);
    go(&mut app, bob, beside_outer);
    act(&mut app, bob, Action::UseOn(outer));
    assert!(door(&mut app, OUTER_DOOR).panel_open);
    drop_held(&mut app, bob);
    let cutters = loose(&mut app, "wirecutters");
    pick_up(&mut app, bob, cutters);
    go(&mut app, bob, beside_outer);
    act(&mut app, bob, Action::UseOn(outer));
    let layout = app.world().resource::<WireLayout>().clone();
    for function in [WireFn::MainPower1, WireFn::BackupPower1] {
        app.world_mut()
            .get_mut::<PendingWires>(bob)
            .unwrap()
            .0
            .push(WireOp::CutOrMend {
                door: outer,
                colour: layout.colour_of(function),
            });
        ticks(&mut app, 1);
    }
    assert!(!door(&mut app, OUTER_DOOR).has_power(), "dead");

    // Alice watches the room's air alarm.
    go(&mut app, alice, ROOM_ALARM.step(rs13_world::Dir::East));
    let room_alarm = {
        let mut q = app.world_mut().query::<(Entity, &GridPos, &AirAlarm)>();
        q.iter(app.world()).find(|(_, p, _)| p.0 == ROOM_ALARM).unwrap().0
    };
    act(&mut app, alice, Action::UseOn(room_alarm));
    ticks(&mut app, 1);
    let before = app.world().get::<AlarmPanelOut>(alice).unwrap().0.clone();
    assert_eq!(before.expect("alarm window").pressure_dkpa, 1013);

    // Bob prises the dead door open: a breach, with the closet door still open behind him.
    drop_held(&mut app, bob);
    let crowbar = loose(&mut app, "crowbar");
    pick_up(&mut app, bob, crowbar);
    go(&mut app, bob, beside_outer);
    assert!(matches!(door(&mut app, INNER_DOOR).state, DoorState::Open { .. }));
    act(&mut app, bob, Action::UseOn(outer));
    assert!(matches!(
        door(&mut app, OUTER_DOOR).state,
        DoorState::Opening { .. }
    ));

    // Everyone watches the pressure drop. The closet alarm seals its area: the dead outer
    // door stays open, but the inner door shuts and bolts and the room keeps most of its air.
    let mut lowest = i64::MAX;
    for _ in 0..30 {
        ticks(&mut app, TICKS_PER_SECOND);
        let view = app.world().get::<AlarmPanelOut>(alice).unwrap().0.clone();
        lowest = lowest.min(i64::from(view.expect("alarm window").pressure_dkpa));
    }
    assert!(lowest < 1013, "the room alarm saw the pressure drop: {lowest}");
    assert!(app.world().resource::<AreaSeals>().0.contains("eng-storage"));
    let sealed = door(&mut app, INNER_DOOR);
    assert_eq!(sealed.state, DoorState::Closed);
    assert!(sealed.bolted && sealed.air_locked);
    assert!(matches!(door(&mut app, OUTER_DOOR).state, DoorState::Open { .. }));
    let closet = app
        .world()
        .resource::<rs13_classic::atmos::StationAtmos>()
        .0
        .pressure_at(CLOSET);
    assert!(closet < 5.0, "the closet is open to space: {closet:.1} kPa");
    // The room lost air through the open closet door until the seal shut it, then holds
    // and its vents top it up.
    let room = pressure_around(&app, alice);
    assert!(room > 0.5 * ONE_ATMOSPHERE, "the room held: {room:.1} kPa");
    assert!(pressure_around(&app, bob) < 5.0, "Bob is in vacuum");
    ticks(&mut app, 20 * TICKS_PER_SECOND);
    let later = pressure_around(&app, alice);
    assert!(later > room, "the room recovers: {room:.1} → {later:.1} kPa");

    // Bob's suit kept him alive and unhurt.
    assert_eq!(app.world().get::<Vitals>(bob).unwrap().stat, MobStat::Conscious);
    assert_eq!(app.world().get::<Body>(bob).unwrap().health(), 100.0);

    // In either language: everything the players were told, and the alarm window.
    let told: Vec<LocMsg> = [alice, bob].into_iter().flat_map(|m| messages(&app, m)).collect();
    let ids: Vec<&str> = told.iter().map(|m| m.id.as_str()).collect();
    for id in ["door-panel-opened", "door-forced", "internals-on"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
    let view = app
        .world()
        .get::<AlarmPanelOut>(alice)
        .unwrap()
        .0
        .clone()
        .unwrap();
    for loc in localizers() {
        for msg in &told {
            let text = loc.format_msg(msg);
            assert!(!text.is_empty() && !text.contains('{'), "{}: {text:?}", msg.id);
        }
        let mut args = Args::new();
        args.set("value", f64::from(view.pressure_dkpa) / 10.0);
        let line = loc.format("alarm-pressure", Some(&args)).unwrap();
        assert!(line.contains(char::is_numeric), "{line}");
        for reading in &view.readings {
            let mut args = Args::new();
            args.set("value", reading.value);
            assert!(!loc.format(&reading.id, Some(&args)).unwrap().is_empty());
        }
    }
}

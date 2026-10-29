//! Power in the engineering chamber. Spec: docs/reference/power.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::Vent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::{APC_CELL_CAPACITY, Apc, AreaPower, Channel, PowerUser, Powernets, Smes};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, Dir, GridPos, TileMap, TilePos};

const TICKS_PER_SECOND: usize = 20;
const ENGINEERING_APC: TilePos = TilePos::new(1, 9, 0);
const STORAGE_APC: TilePos = TilePos::new(20, 9, 0);
const SMES: TilePos = TilePos::new(2, 9, 0);
const INNER_DOOR: TilePos = TilePos::new(16, 6, 0);

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

fn seconds(app: &mut App, s: usize) {
    for _ in 0..s * TICKS_PER_SECOND {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn entity_at<C: Component>(app: &mut App, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query_filtered::<(Entity, &GridPos), With<C>>();
    q.iter(app.world())
        .find(|(_, p)| p.0 == pos)
        .unwrap_or_else(|| panic!("nothing at {pos:?}"))
        .0
}

fn apc(app: &mut App, pos: TilePos) -> Apc {
    let e = entity_at::<Apc>(app, pos);
    app.world().get::<Apc>(e).unwrap().clone()
}

fn set_cell(app: &mut App, pos: TilePos, fraction: f64) {
    let e = entity_at::<Apc>(app, pos);
    app.world_mut().get_mut::<Apc>(e).unwrap().cell = Some((APC_CELL_CAPACITY * fraction, APC_CELL_CAPACITY));
}

fn smes_offline(app: &mut App) {
    let e = entity_at::<Smes>(app, SMES);
    app.world_mut().get_mut::<Smes>(e).unwrap().online = false;
}

fn powered(app: &App, area: &str, channel: Channel) -> bool {
    app.world().resource::<AreaPower>().powered(Some(area), channel)
}

fn give(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

fn find_sprite(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn last_message(app: &App, mob: Entity) -> String {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone()
}

#[test]
fn one_cable_network_feeds_both_apcs_from_the_smes() {
    let mut app = setup();
    seconds(&mut app, 3);
    assert_eq!(apc(&mut app, ENGINEERING_APC).area, "engineering");
    assert_eq!(apc(&mut app, STORAGE_APC).area, "eng-storage");
    let nets = app.world().resource::<Powernets>();
    let net = nets.net_at[&SMES];
    assert_eq!(nets.net_at[&ENGINEERING_APC], net);
    assert_eq!(nets.net_at[&STORAGE_APC], net, "through the wall into the closet");
}

#[test]
fn on_the_grid_the_cells_top_up_and_everything_is_powered() {
    let mut app = setup();
    let start = apc(&mut app, ENGINEERING_APC).cell.unwrap().0;
    seconds(&mut app, 60);
    let cell = apc(&mut app, ENGINEERING_APC).cell.unwrap().0;
    assert!(cell > start, "trickle charge: {start} -> {cell}");
    for area in ["engineering", "eng-storage"] {
        for channel in Channel::ALL {
            assert!(powered(&app, area, channel), "{area} {channel:?}");
        }
    }
    let smes_entity = entity_at::<Smes>(&mut app, SMES);
    let smes = app.world().get::<Smes>(smes_entity).unwrap();
    assert!(smes.charge < smes.capacity, "the SMES paid for it");
}

#[test]
fn off_the_grid_the_cell_drains_and_channels_shed_load_in_luna_order() {
    let mut app = setup();
    smes_offline(&mut app);
    seconds(&mut app, 4);
    let a = apc(&mut app, ENGINEERING_APC).cell.unwrap().0;
    seconds(&mut app, 20);
    let b = apc(&mut app, ENGINEERING_APC).cell.unwrap().0;
    assert!(b < a, "running on the cell: {a} -> {b}");

    // 20%: lighting goes first.
    set_cell(&mut app, ENGINEERING_APC, 0.20);
    seconds(&mut app, 2);
    assert!(!powered(&app, "engineering", Channel::Lighting));
    assert!(powered(&app, "engineering", Channel::Equipment));
    assert!(powered(&app, "engineering", Channel::Environment));

    // 10%: equipment too. Environment holds out to the end.
    set_cell(&mut app, ENGINEERING_APC, 0.10);
    seconds(&mut app, 2);
    assert!(!powered(&app, "engineering", Channel::Equipment));
    assert!(powered(&app, "engineering", Channel::Environment));

    // Empty: everything off.
    set_cell(&mut app, ENGINEERING_APC, 0.0);
    seconds(&mut app, 2);
    assert!(!powered(&app, "engineering", Channel::Environment));
    let vent = entity_at::<Vent>(&mut app, TilePos::new(4, 10, 0));
    assert!(!app.world().get::<PowerUser>(vent).unwrap().powered, "vents stop");
    // The closet has its own APC, still fine.
    assert!(powered(&app, "eng-storage", Channel::Equipment));
}

#[test]
fn an_unpowered_airlock_will_not_open_but_a_crowbar_forces_it() {
    let mut app = setup();
    smes_offline(&mut app);
    set_cell(&mut app, ENGINEERING_APC, 0.10);
    seconds(&mut app, 2);
    let door = entity_at::<Airlock>(&mut app, INNER_DOOR);
    assert!(!app.world().get::<Airlock>(door).unwrap().powered);

    let mob = spawn_player_mob(app.world_mut(), INNER_DOOR.step(Dir::West));
    act(&mut app, mob, Action::UseOn(door));
    seconds(&mut app, 1);
    assert_eq!(
        app.world().get::<Airlock>(door).unwrap().state,
        DoorState::Closed,
        "no power"
    );

    let crowbar = find_sprite(&mut app, "crowbar");
    give(&mut app, mob, crowbar);
    act(&mut app, mob, Action::UseOn(door));
    assert!(matches!(
        app.world().get::<Airlock>(door).unwrap().state,
        DoorState::Opening { .. }
    ));
    assert_eq!(last_message(&app, mob), "door-forced");
}

#[test]
fn the_apc_breaker_needs_engineering_access_and_cuts_the_area() {
    let mut app = setup();
    seconds(&mut app, 2);
    let apc_entity = entity_at::<Apc>(&mut app, STORAGE_APC);
    let mob = spawn_player_mob(app.world_mut(), STORAGE_APC.step(Dir::West));

    act(&mut app, mob, Action::UseOn(apc_entity));
    assert_eq!(last_message(&app, mob), "apc-locked");
    assert!(apc(&mut app, STORAGE_APC).operating);

    let card = {
        let mut q = app.world_mut().query::<(Entity, &IdCard)>();
        q.iter(app.world())
            .find(|(_, c)| c.access.iter().any(|a| a == "engine_equip"))
            .unwrap()
            .0
    };
    give(&mut app, mob, card);
    act(&mut app, mob, Action::UseOn(apc_entity));
    assert_eq!(last_message(&app, mob), "apc-breaker-off");
    seconds(&mut app, 2);
    for channel in Channel::ALL {
        assert!(!powered(&app, "eng-storage", channel));
    }
    assert!(
        powered(&app, "engineering", Channel::Equipment),
        "other areas unaffected"
    );
    assert_eq!(app.world().get::<Appearance>(apc_entity).unwrap().state, "off");
}

#[test]
fn examining_an_apc_shows_its_cell_and_channels() {
    let mut app = setup();
    seconds(&mut app, 2);
    let apc_entity = entity_at::<Apc>(&mut app, ENGINEERING_APC);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    act(&mut app, mob, Action::Examine(apc_entity));
    let ids: Vec<String> = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(ids, ["examine", "examine-apc"]);
}

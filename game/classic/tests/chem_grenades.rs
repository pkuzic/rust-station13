//! Chemical grenades, smoke, foam and foamed metal. Spec: docs/reference/grenades.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::chem_effects::{self, Disfigured, Foam, FoamedMetal, Smoke};
use rs13_classic::chemistry::{Reagents, transfer};
use rs13_classic::devices;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::eyes::EyeBlurry;
use rs13_classic::flashes::Weakened;
use rs13_classic::grenades::{self, ChemGrenade, Grenade};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::pulling::Stepped;
use rs13_simulation::atmos::Cell;
use rs13_world::{Appearance, GridPos, HumanLook, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWWWWW",
        "W.............W",
        "W.............W",
        "W.............W",
        "W.............W",
        "W.............W",
        "W.............W",
        "W.............W",
        "WWWWWWWWWWWWWWW",
    ],
)"#;

const TICKS_PER_DS: u32 = 2;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

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

fn wait_ds(app: &mut App, ds: u32) {
    tick(app, ds * TICKS_PER_DS);
}

fn person(app: &mut App, pos: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), pos);
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

fn spawn(app: &mut App, id: &str, pos: TilePos) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), pos).unwrap()
}

fn give(app: &mut App, mob: Entity, item: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

/// Out of the active hand onto a tile.
fn put_down(app: &mut App, mob: Entity, item: Entity, pos: TilePos) {
    app.world_mut()
        .entity_mut(item)
        .remove::<HeldBy>()
        .insert(GridPos(pos));
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = None;
}

fn wear(app: &mut App, mob: Entity, slot: Slot, id: &str) -> Entity {
    let item = spawn(app, id, at(1, 1));
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(WornBy(mob));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(slot, item);
    item
}

fn gone(app: &App, e: Entity) -> bool {
    app.world().get_entity(e).is_err()
}

fn count<T: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&T>();
    q.iter(app.world()).count()
}

fn state(app: &App, e: Entity) -> String {
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

fn prime_in_hand(app: &mut App, mob: Entity, grenade: Entity) {
    give(app, mob, grenade);
    act(app, mob, Action::UseOn(grenade));
}

#[test]
fn a_casing_is_put_together_step_by_step() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 3));
    let casing = spawn(&mut app, grenades::CASING, at(3, 3));
    // Not ready: it can't be primed.
    prime_in_hand(&mut app, mob, casing);
    assert!(app.world().get::<Grenade>(casing).unwrap().goes_off_ds.is_none());
    put_down(&mut app, mob, casing, at(3, 3));

    let assembly = spawn(&mut app, devices::TIMER_IGNITER, at(3, 3));
    give(&mut app, mob, assembly);
    act(&mut app, mob, Action::UseOn(casing));
    assert!(gone(&app, assembly));
    assert_eq!(app.world().get::<ChemGrenade>(casing).unwrap().stage, 1);
    assert_eq!(state(&app, casing), "assembled");

    // Locking wants a beaker first.
    let screwdriver = spawn(&mut app, "screwdriver", at(3, 3));
    give(&mut app, mob, screwdriver);
    act(&mut app, mob, Action::UseOn(casing));
    assert_eq!(app.world().get::<ChemGrenade>(casing).unwrap().stage, 1);

    // An empty beaker won't go in; a full one does.
    let empty = spawn(&mut app, "beaker", at(3, 3));
    give(&mut app, mob, empty);
    act(&mut app, mob, Action::UseOn(casing));
    assert!(app.world().get::<ChemGrenade>(casing).unwrap().beakers.is_empty());
    app.world_mut()
        .get_mut::<Reagents>(empty)
        .unwrap()
        .add("water", 20.0);
    act(&mut app, mob, Action::UseOn(casing));
    assert_eq!(
        app.world().get::<ChemGrenade>(casing).unwrap().beakers,
        vec![empty]
    );

    give(&mut app, mob, screwdriver);
    act(&mut app, mob, Action::UseOn(casing));
    assert_eq!(app.world().get::<ChemGrenade>(casing).unwrap().stage, 2);
    assert_eq!(state(&app, casing), "locked");

    prime_in_hand(&mut app, mob, casing);
    assert_eq!(state(&app, casing), "active");
    wait_ds(&mut app, 32);
    assert!(gone(&app, casing));
    assert!(gone(&app, empty));
}

#[test]
fn tear_gas_fills_the_room_with_pepper_smoke() {
    let mut app = setup();
    let thrower = person(&mut app, at(7, 4));
    let bystander = person(&mut app, at(7, 5));
    let masked = person(&mut app, at(6, 4));
    wear(&mut app, masked, Slot::Mask, "gas-mask");
    let grenade = spawn(&mut app, grenades::TEARGAS_GRENADE, at(7, 4));
    assert_eq!(state(&app, grenade), "locked");
    prime_in_hand(&mut app, thrower, grenade);
    wait_ds(&mut app, 31);
    assert!(gone(&app, grenade));
    assert_eq!(count::<Smoke>(&mut app), 10);
    wait_ds(&mut app, 11);
    assert_eq!(count::<Smoke>(&mut app), 20);
    wait_ds(&mut app, 40);
    let hit = [thrower, bystander]
        .iter()
        .filter(|m| app.world().get::<EyeBlurry>(**m).is_some())
        .count();
    assert!(hit >= 1, "someone in the smoke got pepper in the eyes");
    assert!(
        app.world().get::<EyeBlurry>(masked).is_none(),
        "a gas mask keeps it out"
    );
    // It thins out in under half a minute.
    wait_ds(&mut app, 200);
    assert_eq!(count::<Smoke>(&mut app), 0);
}

#[test]
fn metal_foam_sets_into_walls_that_hold_air_and_can_be_smashed() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let grenade = spawn(&mut app, grenades::METAL_FOAM_GRENADE, at(8, 4));
    // Primed where it lies.
    give(&mut app, mob, grenade);
    act(&mut app, mob, Action::UseOn(grenade));
    put_down(&mut app, mob, grenade, at(8, 4));
    wait_ds(&mut app, 31);
    assert!(gone(&app, grenade));
    assert!(count::<Foam>(&mut app) >= 1);
    wait_ds(&mut app, 60);
    // 50 units made: it spreads four tiles out.
    let foam = count::<Foam>(&mut app);
    assert!(foam >= 25, "{foam}");
    wait_ds(&mut app, 200);
    assert_eq!(count::<Foam>(&mut app), 0);
    assert!(count::<FoamedMetal>(&mut app) >= 25);
    assert_eq!(
        app.world().resource::<StationAtmos>().0.cell(at(8, 4)),
        Cell::Solid
    );
    // Next to the mob, a wall to smash by hand: half the time for aluminium foam.
    let mut walls = app.world_mut().query::<(Entity, &GridPos, &FoamedMetal)>();
    let wall = walls
        .iter(app.world())
        .find(|(_, p, _)| p.0 == at(5, 4))
        .map(|(e, ..)| e)
        .expect("foamed metal at 5,4");
    app.world_mut().entity_mut(mob).insert(GridPos(at(4, 4)));
    for _ in 0..30 {
        act(&mut app, mob, Action::UseOn(wall));
        if gone(&app, wall) {
            break;
        }
    }
    assert!(gone(&app, wall));
    tick(&mut app, 1);
    assert_eq!(
        app.world().resource::<StationAtmos>().0.cell(at(5, 4)),
        Cell::Open
    );
}

#[test]
fn cleaner_foam_spreads_slips_and_melts_away() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let grenade = spawn(&mut app, grenades::CLEANER_GRENADE, at(8, 4));
    give(&mut app, mob, grenade);
    act(&mut app, mob, Action::UseOn(grenade));
    put_down(&mut app, mob, grenade, at(8, 4));
    wait_ds(&mut app, 45);
    assert!(count::<Foam>(&mut app) >= 5);
    // Walking into it.
    app.world_mut().entity_mut(mob).insert(GridPos(at(8, 4)));
    app.world_mut().resource_mut::<Stepped>().0.push((mob, at(8, 4)));
    chem_effects::slip_on_foam(app.world_mut());
    assert!(app.world().get::<Weakened>(mob).is_some());
    wait_ds(&mut app, 200);
    assert_eq!(count::<Foam>(&mut app), 0);
    assert_eq!(count::<FoamedMetal>(&mut app), 0);
}

#[test]
fn smoke_powder_mixed_in_a_beaker_smokes() {
    let mut app = setup();
    let a = spawn(&mut app, "beaker", at(4, 4));
    let b = spawn(&mut app, "beaker", at(4, 4));
    {
        let mut r = app.world_mut().get_mut::<Reagents>(a).unwrap();
        r.add("potassium", 10.0);
        r.add("sugar", 10.0);
    }
    app.world_mut()
        .get_mut::<Reagents>(b)
        .unwrap()
        .add("phosphorus", 10.0);
    transfer(app.world_mut(), b, a, 10.0);
    assert_eq!(app.world().get::<Reagents>(a).unwrap().total(), 0.0);
    assert_eq!(count::<Smoke>(&mut app), 10);
}

#[test]
fn acid_melts_the_mask_and_scars_the_face() {
    let mut app = setup();
    let mob = person(&mut app, at(4, 4));
    let mask = wear(&mut app, mob, Slot::Mask, "gas-mask");
    chem_effects::touch(app.world_mut(), mob, "pacid");
    assert!(gone(&app, mask));
    assert!(app.world().get::<Disfigured>(mob).is_some());
    // Sulphuric acid: a hat takes it instead.
    let other = person(&mut app, at(5, 4));
    let hat = wear(&mut app, other, Slot::Head, "helmet");
    chem_effects::touch(app.world_mut(), other, "sacid");
    assert!(gone(&app, hat));
    assert!(app.world().get::<Disfigured>(other).is_none());
}

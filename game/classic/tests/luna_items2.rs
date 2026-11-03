//! The rest of Luna's simple items that do something. Spec: docs/reference/items.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::equipment::TankGas;
use rs13_classic::food::{Nutrition, Snack};
use rs13_classic::health::{Body, Zone};
use rs13_classic::machines::{GasStore, is_canister};
use rs13_classic::medical::heals;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::Gas;
use rs13_world::{ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn spawn(app: &mut App, proto: &str) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).expect(proto)
}

#[test]
fn a_plasma_tank_is_full_of_plasma_at_three_atmospheres() {
    let mut app = setup();
    let tank = spawn(&mut app, "plasma-tank");
    let gas = app.world().get::<TankGas>(tank).unwrap();
    assert!(gas.mix.get(Gas::Plasma) > 0.0);
    assert_eq!(gas.mix.get(Gas::Oxygen), 0.0);
    assert!((gas.mix.pressure(gas.volume) - 303.975).abs() < 1.0);
}

#[test]
fn an_empty_canister_is_a_canister_with_nothing_in_it() {
    let mut app = setup();
    assert!(is_canister("empty-canister"));
    let can = spawn(&mut app, "empty-canister");
    assert_eq!(app.world().get::<GasStore>(can).unwrap().0.total_moles(), 0.0);
}

#[test]
fn advanced_kits_heal_more_than_the_plain_ones() {
    assert!(heals("advanced-trauma-kit").unwrap().0 > heals("bruise-pack").unwrap().0);
    assert!(heals("advanced-burn-kit").unwrap().1 > heals("ointment").unwrap().1);
    let mut app = setup();
    let medic = spawn_player_mob(app.world_mut(), AT);
    let patient = spawn_player_mob(app.world_mut(), AT);
    app.world_mut()
        .get_mut::<Body>(patient)
        .unwrap()
        .take_damage(Zone::Chest, 80.0, 0.0);
    let kit = spawn(&mut app, "advanced-trauma-kit");
    app.world_mut().run_schedule(FixedUpdate);
    for target in [kit, patient] {
        app.world_mut()
            .get_mut::<PendingActions>(medic)
            .unwrap()
            .0
            .push(Action::UseOn(target));
        app.world_mut().run_schedule(FixedUpdate);
    }
    let brute = app.world().get::<Body>(patient).unwrap().organ(Zone::Chest).brute;
    assert!((brute - 20.0).abs() < 0.01, "{brute}");
}

#[test]
fn a_pill_bottle_comes_full_of_pills_with_medicine_in() {
    let mut app = setup();
    let pill = spawn(&mut app, "pill-kelotane");
    assert!(app.world().get::<Reagents>(pill).unwrap().get("kelotane") > 0.0);
    spawn(&mut app, "pill-bottle-kelotane");
    let mut pills = app.world_mut().query::<&Reagents>();
    let with_kelotane = pills
        .iter(app.world())
        .filter(|r| r.get("kelotane") > 0.0)
        .count();
    assert_eq!(with_kelotane, 8, "the one, and the bottle's 7");
}

#[test]
fn new_food_can_be_eaten() {
    for proto in [
        "banana",
        "plump-helmet",
        "waffles",
        "noodles",
        "candy-cane",
        "fishburger",
    ] {
        let mut app = setup();
        let mob = spawn_player_mob(app.world_mut(), AT);
        let food = spawn(&mut app, proto);
        app.world_mut().run_schedule(FixedUpdate);
        let before = app.world().get::<Snack>(food).expect(proto).bites_left;
        let fed = app.world().get::<Nutrition>(mob).unwrap().level;
        for target in [food, mob] {
            app.world_mut()
                .get_mut::<PendingActions>(mob)
                .unwrap()
                .0
                .push(Action::UseOn(target));
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert_eq!(
            app.world().get::<Snack>(food).unwrap().bites_left,
            before - 1,
            "{proto}"
        );
        assert!(
            app.world().get::<Nutrition>(mob).unwrap().level > fed - 1.0,
            "{proto}"
        );
    }
}

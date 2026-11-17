//! Hacking vending machines, the autolathe and APCs. Spec: docs/reference/hacking.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::autolathe::{HIDDEN, LathePanelOut, RECIPES};
use rs13_classic::grenades::Flying;
use rs13_classic::hacking::{APC_SHORT_DS, APC_UNLOCK_DS, LATHE_PULSE_DS, MachineWires, Wire};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::{Apc, PowerUser};
use rs13_classic::vending::{PendingVendOps, Vending, VendingPanelOut};
use rs13_classic::wires::{PendingWires, WireOp, WirePanelOut};
use rs13_world::{GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWW",
        "W.....W",
        "W.....W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(2, 2, 0);
const NEXT: TilePos = TilePos::new(3, 2, 0);

fn setup(machine: &str) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let machine = spawn_object(app.world_mut(), &ObjectSpec::new(machine), NEXT).unwrap();
    tick(&mut app, 1);
    power(&mut app, machine);
    (app, mob, machine)
}

/// No APC in this room: power it by hand.
fn power(app: &mut App, machine: Entity) {
    if let Some(mut p) = app.world_mut().get_mut::<PowerUser>(machine) {
        p.powered = true;
    }
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn seconds(app: &mut App, machine: Entity, ds: f32) {
    for _ in 0..(ds * 2.0) as u32 {
        power(app, machine);
        tick(app, 1);
    }
}

fn hold(app: &mut App, mob: Entity, id: &str) {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(id), HERE).unwrap();
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    if let Some(old) = app
        .world_mut()
        .get_mut::<Hands>(mob)
        .unwrap()
        .slot_mut(hand)
        .take()
    {
        app.world_mut().entity_mut(old).despawn();
    }
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

fn empty_hand(app: &mut App, mob: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    if let Some(old) = app
        .world_mut()
        .get_mut::<Hands>(mob)
        .unwrap()
        .slot_mut(hand)
        .take()
    {
        app.world_mut().entity_mut(old).despawn();
    }
}

fn use_on(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    power(app, target);
    tick(app, 2);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn wires(app: &App, machine: Entity) -> MachineWires {
    app.world().get::<MachineWires>(machine).unwrap().clone()
}

fn op(app: &mut App, mob: Entity, machine: Entity, wire: Wire, pulse: bool) {
    let colour = wires(app, machine).colour_of(wire);
    let op = if pulse {
        WireOp::Pulse {
            door: machine,
            colour,
        }
    } else {
        WireOp::CutOrMend {
            door: machine,
            colour,
        }
    };
    app.world_mut().get_mut::<PendingWires>(mob).unwrap().0.push(op);
    power(app, machine);
    tick(app, 2);
}

/// Open the panel and look at the wires with the tool to work them.
fn open_panel(app: &mut App, mob: Entity, machine: Entity, tool: &str) {
    hold(app, mob, "screwdriver");
    use_on(app, mob, machine);
    assert!(wires(app, machine).panel_open);
    hold(app, mob, tool);
    use_on(app, mob, machine);
}

#[test]
fn a_vending_panel_shows_four_wires_and_contraband_follows_its_wire() {
    let (mut app, mob, machine) = setup("vending-cola");
    open_panel(&mut app, mob, machine, "multitool");
    let view = app.world().get::<WirePanelOut>(mob).unwrap().0.clone().unwrap();
    let colours: Vec<&str> = view.wires.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(colours, ["violet", "orange", "goldenrod", "green"]);
    assert!(view.lights.contains(&"light-green-off".to_owned()));

    op(&mut app, mob, machine, Wire::Contraband, true);
    assert!(wires(&app, machine).contraband);
    // The machine's own window now lists root beer at the end.
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, machine);
    let rows = app
        .world()
        .get::<VendingPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .unwrap()
        .products;
    assert_eq!(rows.last().unwrap().item, "rootbeer");
    let line = rows.len() - 1;
    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, line));
    seconds(&mut app, machine, 20.0);
    let mut q = app.world_mut().query::<(&rs13_world::Appearance, &GridPos)>();
    assert!(
        q.iter(app.world())
            .any(|(a, p)| a.sprite == "rootbeer" && p.0 == NEXT)
    );

    // Cutting it hides the contraband again.
    hold(&mut app, mob, "wirecutters");
    op(&mut app, mob, machine, Wire::Contraband, false);
    assert!(!wires(&app, machine).contraband);
}

#[test]
fn the_wrong_tool_or_a_cut_wire_does_nothing() {
    let (mut app, mob, machine) = setup("vending-cola");
    open_panel(&mut app, mob, machine, "wirecutters");
    op(&mut app, mob, machine, Wire::Throw, true);
    assert!(!wires(&app, machine).throwing);
    let said = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone();
    assert_eq!(said, "wire-need-multitool");
    op(&mut app, mob, machine, Wire::Throw, false);
    assert!(wires(&app, machine).throwing, "a cut throw wire sets it throwing");
    hold(&mut app, mob, "multitool");
    op(&mut app, mob, machine, Wire::Throw, true);
    assert!(wires(&app, machine).throwing, "a cut wire can't be pulsed");
}

#[test]
fn a_broken_machine_throws_its_stock_at_people() {
    let (mut app, _mob, machine) = setup("vending-snack");
    let before: u16 = app.world().get::<Vending>(machine).unwrap().stock.iter().sum();
    assert!(rs13_classic::hacking::throw_item(app.world_mut(), machine));
    let after: u16 = app.world().get::<Vending>(machine).unwrap().stock.iter().sum();
    assert_eq!(after, before - 1);
    let mut q = app.world_mut().query::<&Flying>();
    assert_eq!(q.iter(app.world()).count(), 1, "something is flying");
}

#[test]
fn a_cut_id_wire_lets_anyone_buy_from_a_secure_machine() {
    let (mut app, mob, machine) = setup("vending-medical");
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, machine);
    let stock = |app: &App| -> u16 { app.world().get::<Vending>(machine).unwrap().stock[0] };
    let full = stock(&app);
    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, 0));
    tick(&mut app, 2);
    assert_eq!(stock(&app), full, "no ID, no sale");

    open_panel(&mut app, mob, machine, "wirecutters");
    op(&mut app, mob, machine, Wire::ScanId, false);
    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, 0));
    power(&mut app, machine);
    tick(&mut app, 2);
    assert_eq!(stock(&app), full - 1);
}

#[test]
fn the_autolathe_hack_wire_shows_the_hidden_list_for_ten_seconds() {
    let (mut app, mob, machine) = setup("autolathe");
    open_panel(&mut app, mob, machine, "multitool");
    assert!(
        app.world()
            .get::<WirePanelOut>(mob)
            .unwrap()
            .0
            .as_ref()
            .unwrap()
            .wires
            .len()
            == 10
    );
    op(&mut app, mob, machine, Wire::Hack, true);
    assert!(wires(&app, machine).hacked);

    // Shut the hatch and open the window.
    hold(&mut app, mob, "screwdriver");
    use_on(&mut app, mob, machine);
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, machine);
    let rows = |app: &App| {
        app.world()
            .get::<LathePanelOut>(mob)
            .unwrap()
            .0
            .as_ref()
            .map(|v| v.rows.len())
    };
    assert_eq!(rows(&app), Some(RECIPES.len() + HIDDEN.len()));
    seconds(&mut app, machine, LATHE_PULSE_DS);
    assert!(!wires(&app, machine).hacked);
    assert_eq!(rows(&app), Some(RECIPES.len()));
}

#[test]
fn a_cut_disable_wire_kills_the_autolathe_buttons() {
    let (mut app, mob, machine) = setup("autolathe");
    open_panel(&mut app, mob, machine, "wirecutters");
    op(&mut app, mob, machine, Wire::Disable, false);
    hold(&mut app, mob, "screwdriver");
    use_on(&mut app, mob, machine);
    empty_hand(&mut app, mob);
    let said = use_on(&mut app, mob, machine);
    assert!(said.contains(&"lathe-dead".to_owned()), "{said:?}");
    assert!(app.world().get::<LathePanelOut>(mob).unwrap().0.is_none());
}

#[test]
fn apc_main_wires_short_it_and_the_id_wire_unlocks_it() {
    let (mut app, mob, machine) = setup("apc");
    hold(&mut app, mob, "screwdriver");
    let said = use_on(&mut app, mob, machine);
    assert!(said.contains(&"apc-wires-exposed".to_owned()), "{said:?}");
    hold(&mut app, mob, "multitool");
    use_on(&mut app, mob, machine);
    let view = app.world().get::<WirePanelOut>(mob).unwrap().0.clone().unwrap();
    assert!(view.lights.contains(&"apc-panel-locked".to_owned()));

    op(&mut app, mob, machine, Wire::ApcIdScan, true);
    assert!(!app.world().get::<Apc>(machine).unwrap().locked);
    op(&mut app, mob, machine, Wire::ApcMain1, true);
    assert!(app.world().get::<Apc>(machine).unwrap().shorted);
    seconds(&mut app, machine, APC_UNLOCK_DS);
    assert!(app.world().get::<Apc>(machine).unwrap().locked, "it locks again");
    seconds(&mut app, machine, APC_SHORT_DS);
    assert!(!app.world().get::<Apc>(machine).unwrap().shorted);

    // A cut main wire keeps it shorted until both are whole.
    hold(&mut app, mob, "wirecutters");
    op(&mut app, mob, machine, Wire::ApcMain2, false);
    assert!(app.world().get::<Apc>(machine).unwrap().shorted);
    op(&mut app, mob, machine, Wire::ApcMain2, false);
    assert!(!app.world().get::<Apc>(machine).unwrap().shorted);
}

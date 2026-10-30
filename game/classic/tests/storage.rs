//! Toolboxes and belts. Spec: docs/reference/storage.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::{EquipAction, Equipment, PendingEquipment};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::storage::{InStorage, Storage, StorageOut};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, GridPos, ObjectSpec, Slot, TileMap, TilePos};

const SPOT: TilePos = TilePos::new(8, 8, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), SPOT);
    (app, mob)
}

fn spawn(app: &mut App, proto: &str) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(proto), SPOT).unwrap()
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

fn contents(app: &App, container: Entity) -> Vec<String> {
    app.world()
        .get::<Storage>(container)
        .unwrap()
        .items
        .iter()
        .map(|i| app.world().get::<Appearance>(*i).unwrap().sprite.clone())
        .collect()
}

fn shown(app: &App, mob: Entity) -> Option<usize> {
    app.world()
        .get::<StorageOut>(mob)
        .unwrap()
        .0
        .as_ref()
        .map(|v| v.items.len())
}

#[test]
fn a_toolbox_comes_with_its_tools_and_opens_from_the_hand() {
    let (mut app, mob) = setup();
    let toolbox = spawn(&mut app, "toolbox-mechanical");
    assert_eq!(
        contents(&app, toolbox),
        ["screwdriver", "wrench", "welder", "crowbar", "wirecutters"]
    );
    let crowbar = app.world().get::<Storage>(toolbox).unwrap().items[3];
    assert!(
        app.world().get::<GridPos>(crowbar).is_none(),
        "inside, not on the floor"
    );
    assert_eq!(app.world().get::<InStorage>(crowbar), Some(&InStorage(toolbox)));

    // From the floor, an empty hand picks it up rather than opening it.
    act(&mut app, mob, Action::UseOn(toolbox));
    assert_eq!(active(&app, mob), Some(toolbox));
    assert_eq!(shown(&app, mob), None);

    // Clicked with the other, empty hand, it opens.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(toolbox));
    assert_eq!(shown(&app, mob), Some(5));

    // Clicking a tool in it takes it out.
    act(&mut app, mob, Action::UseOn(crowbar));
    assert_eq!(active(&app, mob), Some(crowbar));
    assert_eq!(app.world().get::<HeldBy>(crowbar), Some(&HeldBy(mob)));
    assert_eq!(shown(&app, mob), Some(4));

    // And clicking the toolbox with it puts it back, in plain view.
    act(&mut app, mob, Action::UseOn(toolbox));
    assert_eq!(active(&app, mob), None);
    assert_eq!(
        contents(&app, toolbox).last().map(String::as_str),
        Some("crowbar")
    );
    let told = &app.world().get::<Outbox>(mob).unwrap().0;
    assert!(
        told.iter().any(|m| m.id == "storage-added"),
        "everyone around sees it"
    );

    // Putting it down closes it.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::Drop);
    assert_eq!(shown(&app, mob), None);
}

#[test]
fn a_toolbox_takes_only_smaller_things_and_only_seven() {
    let (mut app, mob) = setup();
    let toolbox = spawn(&mut app, "toolbox-electrical");
    let suit = spawn(&mut app, "space-suit");
    act(&mut app, mob, Action::UseOn(suit));
    act(&mut app, mob, Action::UseOn(toolbox));
    assert_eq!(
        active(&app, mob),
        Some(suit),
        "a space suit is as big as a toolbox"
    );
    assert_eq!(
        contents(&app, toolbox),
        [
            "screwdriver",
            "wirecutters",
            "crowbar",
            "cable-coil",
            "cable-coil",
            "cable-coil"
        ]
    );
    act(&mut app, mob, Action::Drop);

    for _ in 0..2 {
        let card = spawn(&mut app, "id-card");
        act(&mut app, mob, Action::UseOn(card));
        act(&mut app, mob, Action::UseOn(toolbox));
    }
    assert_eq!(contents(&app, toolbox).len(), 7);
    assert!(active(&app, mob).is_some(), "the eighth card stays in hand");
}

#[test]
fn a_worn_belt_holds_only_tools() {
    let (mut app, mob) = setup();
    let belt = spawn(&mut app, "utility-belt-loaded");
    assert_eq!(
        app.world().get::<Appearance>(belt).unwrap().sprite,
        "utility-belt"
    );
    act(&mut app, mob, Action::UseOn(belt));
    app.world_mut()
        .get_mut::<PendingEquipment>(mob)
        .unwrap()
        .0
        .push(EquipAction::Equip(Slot::Belt));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(Slot::Belt),
        Some(belt)
    );

    // Clicking the worn belt with an empty hand opens it.
    act(&mut app, mob, Action::UseOn(belt));
    assert_eq!(shown(&app, mob), Some(6));

    let card = spawn(&mut app, "id-card");
    act(&mut app, mob, Action::UseOn(card));
    act(&mut app, mob, Action::UseOn(belt));
    assert_eq!(active(&app, mob), Some(card));
    let told = &app.world().get::<Outbox>(mob).unwrap().0;
    assert_eq!(told.last().unwrap().id, "storage-cannot-hold");
    act(&mut app, mob, Action::Drop);

    let multitool = spawn(&mut app, "multitool");
    act(&mut app, mob, Action::UseOn(multitool));
    act(&mut app, mob, Action::UseOn(belt));
    assert_eq!(shown(&app, mob), Some(7));
}

#[test]
fn clicking_what_you_carry_with_an_empty_hand() {
    let (mut app, mob) = setup();
    // Worn clothes come off into the hand.
    let shoes = app
        .world()
        .get::<Equipment>(mob)
        .unwrap()
        .get(Slot::Shoes)
        .unwrap();
    act(&mut app, mob, Action::UseOn(shoes));
    assert_eq!(active(&app, mob), Some(shoes));
    assert_eq!(app.world().get::<Equipment>(mob).unwrap().get(Slot::Shoes), None);

    // An item in the other hand moves to the active one.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(shoes));
    assert_eq!(active(&app, mob), Some(shoes));
    let hands = app.world().get::<Hands>(mob).unwrap();
    assert_eq!(hands.get(hands.active.other()), None);
}

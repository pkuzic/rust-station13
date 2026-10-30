//! Clothing, internals and space suits in the engineering chamber.
//! Spec: docs/reference/equipment.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::Outbox;
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::environment::Lungs;
use rs13_classic::equipment::{EquipAction, Equipment, PendingEquipment, TankGas};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Vitals};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, Dir, GridPos, MobStat, Slot, TileMap, TilePos, WornView};

const TICKS_PER_SECOND: usize = 20;
const OUTER_DOOR: TilePos = TilePos::new(20, 6, 0);
/// Out of the wind's path in the closet.
const CLOSET_CORNER: TilePos = TilePos::new(17, 10, 0);

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

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance, Option<&GridPos>)>();
    q.iter(app.world())
        .find(|(_, a, p)| a.sprite == sprite && p.is_some())
        .unwrap_or_else(|| panic!("no loose {sprite}"))
        .0
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
}

fn equip(app: &mut App, mob: Entity, action: EquipAction) {
    app.world_mut()
        .get_mut::<PendingEquipment>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 1);
}

/// Pick `sprite` up off the floor and put it on.
fn wear(app: &mut App, mob: Entity, sprite: &str, slot: Slot) {
    let item = find(app, sprite);
    hold(app, mob, item);
    equip(app, mob, EquipAction::Equip(slot));
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(slot),
        Some(item),
        "{sprite}"
    );
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

fn open_outer_door(app: &mut App) {
    let mut q = app.world_mut().query::<(&GridPos, &mut Airlock)>();
    for (pos, mut door) in q.iter_mut(app.world_mut()) {
        if pos.0 == OUTER_DOOR {
            door.state = DoorState::Open { autoclose_at: None };
        }
    }
}

#[test]
fn new_crew_start_in_a_jumpsuit_and_shoes() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    ticks(&mut app, 1);
    let equipment = app.world().get::<Equipment>(mob).unwrap();
    assert!(equipment.get(Slot::Uniform).is_some());
    assert!(equipment.get(Slot::Shoes).is_some());
    let view = app.world().get::<WornView>(mob).unwrap();
    assert!(view.get(Slot::Uniform).is_some() && view.get(Slot::Shoes).is_some());
}

#[test]
fn the_id_hangs_off_the_jumpsuit() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(9, 4, 0));
    wear(&mut app, mob, "id-card", Slot::Id);

    // Wrong slot.
    let crowbar = find(&mut app, "crowbar");
    hold(&mut app, mob, crowbar);
    equip(&mut app, mob, EquipAction::Equip(Slot::Head));
    assert_eq!(last_message(&app, mob), "equip-wrong-slot");
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    app.world_mut()
        .entity_mut(crowbar)
        .remove::<HeldBy>()
        .insert(GridPos(TilePos::new(9, 4, 0)));

    // Taking the jumpsuit off drops the ID on the floor.
    equip(&mut app, mob, EquipAction::Unequip(Slot::Uniform));
    let equipment = app.world().get::<Equipment>(mob).unwrap();
    assert!(equipment.get(Slot::Id).is_none());
    assert!(equipment.get(Slot::Uniform).is_none());
    let card = {
        let mut q = app.world_mut().query::<(Entity, &Appearance, &GridPos)>();
        q.iter(app.world())
            .find(|(_, a, _)| a.sprite == "id-card")
            .map(|(e, ..)| e)
    };
    assert!(card.is_some(), "the ID card is on the floor");

    // And without a jumpsuit it cannot go back on.
    let card = card.unwrap();
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    hold(&mut app, mob, card);
    equip(&mut app, mob, EquipAction::Equip(Slot::Id));
    assert_eq!(last_message(&app, mob), "equip-needs-uniform");
}

#[test]
fn a_space_suit_slows_you_down() {
    let step_ticks = |suit: bool| {
        let mut app = setup();
        let mob = spawn_player_mob(app.world_mut(), TilePos::new(17, 8, 0));
        if suit {
            wear(&mut app, mob, "space-suit", Slot::Suit);
        }
        app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(3, 3, 0);
        app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
        let start = app.world().get::<GridPos>(mob).unwrap().0.x;
        // 1.5 s: short of the far wall of the room.
        ticks(&mut app, 30);
        app.world().get::<GridPos>(mob).unwrap().0.x - start
    };
    let (plain, suited) = (step_ticks(false), step_ticks(true));
    // Shoes take 1 ds off: 2.3 - 1 = 1.3 ds -> a step every 3 ticks. Suit adds 3 ds:
    // 4.3 ds -> every 9 ticks.
    assert!(plain > suited * 2, "plain {plain} tiles vs suited {suited}");
}

#[test]
fn internals_need_a_mask_and_a_tank() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(18, 7, 0));
    equip(&mut app, mob, EquipAction::ToggleInternals);
    assert_eq!(last_message(&app, mob), "internals-no-mask");
    wear(&mut app, mob, "breath-mask", Slot::Mask);
    equip(&mut app, mob, EquipAction::ToggleInternals);
    assert_eq!(last_message(&app, mob), "internals-no-tank");
    wear(&mut app, mob, "emergency-oxygen", Slot::Belt);
    equip(&mut app, mob, EquipAction::ToggleInternals);
    assert_eq!(last_message(&app, mob), "internals-on");
    assert!(app.world().get::<Lungs>(mob).unwrap().internal.is_some());

    // Taking the mask off disconnects them.
    equip(&mut app, mob, EquipAction::Unequip(Slot::Mask));
    assert!(app.world().get::<Lungs>(mob).unwrap().internal.is_none());
}

#[test]
fn internals_keep_you_breathing_in_vacuum_but_not_whole() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(18, 7, 0));
    wear(&mut app, mob, "breath-mask", Slot::Mask);
    wear(&mut app, mob, "emergency-oxygen", Slot::Belt);
    equip(&mut app, mob, EquipAction::ToggleInternals);
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = CLOSET_CORNER;
    let tank = app
        .world()
        .get::<Equipment>(mob)
        .unwrap()
        .get(Slot::Belt)
        .unwrap();
    let full = app.world().get::<TankGas>(tank).unwrap().pressure();

    open_outer_door(&mut app);
    ticks(&mut app, 40 * TICKS_PER_SECOND);
    let body = app.world().get::<Body>(mob).unwrap();
    assert!(body.oxy < 1.0, "breathing from the tank: oxy {}", body.oxy);
    assert!(body.health() < 100.0, "but vacuum still hurts without a suit");
    assert!(
        app.world().get::<TankGas>(tank).unwrap().pressure() < full,
        "the tank is being used"
    );
}

#[test]
fn full_eva_gear_survives_a_blowout() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(18, 7, 0));
    wear(&mut app, mob, "space-suit", Slot::Suit);
    wear(&mut app, mob, "space-helmet", Slot::Head);
    wear(&mut app, mob, "breath-mask", Slot::Mask);
    wear(&mut app, mob, "emergency-oxygen", Slot::Belt);
    equip(&mut app, mob, EquipAction::ToggleInternals);
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = CLOSET_CORNER;

    open_outer_door(&mut app);
    ticks(&mut app, 120 * TICKS_PER_SECOND);
    assert_eq!(app.world().get::<Vitals>(mob).unwrap().stat, MobStat::Conscious);
    let body = app.world().get::<Body>(mob).unwrap();
    assert_eq!(
        body.health(),
        100.0,
        "sealed: no pressure damage, no cold, no suffocation"
    );
    let lungs = app.world().get::<Lungs>(mob).unwrap();
    assert!(
        lungs.body_temperature > 300.0,
        "insulated: {}",
        lungs.body_temperature
    );
}

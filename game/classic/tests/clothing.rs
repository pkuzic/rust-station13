//! Luna's clothing, worn for its looks. Spec: docs/reference/equipment.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::equipment::{EquipAction, Equipment, PendingEquipment, Wearable};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWW", "W..W", "WWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn wears(app: &mut App, proto: &str) -> Slot {
    let mob = spawn_player_mob(app.world_mut(), AT);
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    let slot = app.world().get::<Wearable>(item).expect(proto).slot;
    // Take off what the starting outfit has there, as a player would.
    app.world_mut().get_mut::<Equipment>(mob).unwrap().0.remove(&slot);
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    app.world_mut()
        .get_mut::<PendingEquipment>(mob)
        .unwrap()
        .0
        .push(EquipAction::Equip(slot));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(slot),
        Some(item),
        "{proto}"
    );
    assert_eq!(
        app.world().get::<Hands>(mob).unwrap().active_item(),
        None,
        "{proto}"
    );
    slot
}

#[test]
fn each_kind_of_clothes_goes_where_it_should() {
    let mut app = setup();
    for (proto, slot) in [
        ("earmuffs", Slot::Ears),
        ("eyepatch", Slot::Glasses),
        ("latex-gloves", Slot::Gloves),
        ("beret", Slot::Head),
        ("muzzle", Slot::Mask),
        ("jackboots", Slot::Shoes),
        ("armor-vest", Slot::Suit),
        ("kilt", Slot::Uniform),
    ] {
        assert_eq!(wears(&mut app, proto), slot);
    }
}

#[test]
fn all_of_them_can_be_made() {
    let mut app = setup();
    for proto in include_str!("clothing_ids.txt").lines() {
        let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).expect(proto);
        assert!(app.world().get::<Wearable>(item).is_some(), "{proto}");
    }
}

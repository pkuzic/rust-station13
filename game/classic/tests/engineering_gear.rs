//! Engineering gear: what each piece does when worn. Spec: docs/reference/equipment.md.

use bevy_app::App;
use rs13_classic::ClassicPlugin;
use rs13_classic::equipment::Wearable;
use rs13_classic::objects::spawn_object;
use rs13_world::{ObjectSpec, Slot, TilePos};

fn wearable(proto: &str) -> Wearable {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), TilePos::new(0, 0, 0)).unwrap();
    *app.world().get::<Wearable>(item).unwrap()
}

#[test]
fn gear_is_worn_where_luna_wears_it() {
    let boots = wearable("magboots");
    assert_eq!(boots.slot, Slot::Shoes);
    assert!(boots.magnetic);
    assert_eq!(boots.slowdown, -1.0, "Luna shoes' default");

    // Luna's plain gloves let half a shock through; insulated ones none.
    let gloves = wearable("black-gloves");
    assert_eq!((gloves.slot, gloves.siemens), (Slot::Gloves, 0.5));
    assert_eq!(wearable("insulated-gloves").siemens, 0.0);

    assert_eq!(wearable("meson-glasses").slot, Slot::Glasses);
    assert_eq!(wearable("headset-eng").slot, Slot::Ears);

    let mask = wearable("gas-mask");
    assert_eq!(mask.slot, Slot::Mask);
    assert!(mask.internals, "takes air from a tank like a breath mask");

    let suit = wearable("firesuit");
    assert_eq!((suit.slot, suit.slowdown), (Slot::Suit, 1.0));
    assert!(!suit.space, "no protection from vacuum");

    for hat in [
        "hardhat",
        "hardhat-orange",
        "hardhat-red",
        "hardhat-white",
        "hardhat-blue",
    ] {
        assert_eq!(wearable(hat).slot, Slot::Head, "{hat}");
    }
}

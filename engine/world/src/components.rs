//! ECS components for things placed on the grid.

use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

use crate::{TileMap, TilePos};

/// The tile an entity stands on. Absent while the entity is inside something
/// (held in a hand, in a container).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPos(pub TilePos);

/// One deck of the map, held by a single map entity.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapLayer(pub TileMap);

/// Stable id of a replicated entity. Clients name entities by `NetId` when they send
/// intentions ("use my item on NetId 42"), never by ECS entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetId(pub u64);

/// How an entity looks: a sprite id (also its prototype and noun id) and a state,
/// like BYOND's `icon` and `icon_state`.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appearance {
    pub sprite: String,
    pub state: String,
}

impl Appearance {
    pub fn new(sprite: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            sprite: sprite.into(),
            state: state.into(),
        }
    }
}

/// Seen by no one and out of reach while present, such as a pipe or cable under floor
/// tiles (BYOND `invisibility = 101`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hidden;

/// Blocks movement into its tile while present (BYOND `density`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dense;

/// Blocks sight through its tile while present (BYOND `opacity`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opaque;

/// Gives off light while present: red, green and blue strengths, in tiles of reach
/// (Luna `ul_SetLuminosity`). See `rs13_simulation::lighting`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightSource(pub [u8; 3]);

/// A person's name, as others see it. English only, never declined.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayName(pub String);

/// Whether a mob is awake, out cold or dead (BYOND `stat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MobStat {
    #[default]
    Conscious,
    Unconscious,
    Dead,
}

/// How a mob acts on others (BYOND `a_intent`). Disarm and grab come later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Intent {
    #[default]
    Help,
    Harm,
}

/// A danger the player's own HUD warns about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvAlert {
    /// Hazardously low pressure: hurts every couple of seconds.
    Vacuum,
    LowPressure,
    HighPressure,
    LowOxygen,
    Cold,
    Hot,
}

/// Pressure of every tile of the deck in tenths of a kPa, row-major like the map. For
/// the atmos debug overlay.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AtmosView {
    pub pressure_dkpa: Vec<u16>,
}

/// Which hand is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Hand {
    Left,
    #[default]
    Right,
}

impl Hand {
    pub fn other(self) -> Self {
        match self {
            Hand::Left => Hand::Right,
            Hand::Right => Hand::Left,
        }
    }
}

/// What a mob holds, as seen by clients.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HandsView {
    pub left: Option<NetId>,
    pub right: Option<NetId>,
    pub active: Hand,
}

/// Where something can be worn on a person (BYOND equipment slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    Uniform,
    Suit,
    Head,
    Mask,
    Gloves,
    Shoes,
    Back,
    Belt,
    Id,
    Glasses,
    Ears,
}

impl Slot {
    pub const ALL: [Slot; 11] = [
        Slot::Uniform,
        Slot::Suit,
        Slot::Head,
        Slot::Mask,
        Slot::Gloves,
        Slot::Shoes,
        Slot::Back,
        Slot::Belt,
        Slot::Id,
        Slot::Glasses,
        Slot::Ears,
    ];

    /// Name used in content files and message ids, e.g. `uniform`.
    pub fn id(self) -> &'static str {
        match self {
            Slot::Uniform => "uniform",
            Slot::Suit => "suit",
            Slot::Head => "head",
            Slot::Mask => "mask",
            Slot::Gloves => "gloves",
            Slot::Shoes => "shoes",
            Slot::Back => "back",
            Slot::Belt => "belt",
            Slot::Id => "id",
            Slot::Glasses => "glasses",
            Slot::Ears => "ears",
        }
    }

    pub fn parse(s: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|slot| slot.id() == s)
    }
}

/// An open airlock wire panel, as shown to the player using it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePanelView {
    /// The door.
    pub target: NetId,
    /// Wire colour id and whether it is cut, in panel order.
    pub wires: Vec<(String, bool)>,
    /// Indicator lights, as message ids.
    pub lights: Vec<String>,
}

/// An open air alarm, as shown to the player using it (Luna's alarm window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmPanelView {
    /// The alarm.
    pub target: NetId,
    /// Area id: the client shows the `area-<id>` message.
    pub area: String,
    /// Room pressure, tenths of a kPa, and whether it is out of the normal range.
    pub pressure_dkpa: u32,
    pub pressure_alert: bool,
    /// Other readouts in panel order.
    pub readings: Vec<AlarmReading>,
    /// The area's emergency seal is on.
    pub sealed: bool,
    /// Panic fill on the area's vents; `None` when the area has no vent.
    pub panic_fill: Option<bool>,
    /// The viewer is close enough to press the buttons.
    pub controls: bool,
}

/// An open APC window, as shown to the player using it (Luna's "apc" window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApcPanelView {
    /// The APC.
    pub target: NetId,
    /// Area id: the client shows the `area-<id>` message.
    pub area: String,
    /// The interface is locked: settings show, but only an ID card swipe unlocks them.
    pub locked: bool,
    /// The main breaker is on.
    pub breaker: bool,
    /// Power from the grid this cycle: 0 none, 1 low, 2 good (Luna `main_status`).
    pub external: u8,
    /// Cell charge, percent; `None` with no cell fitted.
    pub cell_percent: Option<u8>,
    /// 0 not charging, 1 charging, 2 fully charged (Luna `charging`).
    pub charging: u8,
    /// The cell charges automatically (Luna `chargemode`).
    pub charge_auto: bool,
    /// Equipment, lighting and environment: mode (Luna 0 off, 1 auto-off, 2 on, 3 auto-on)
    /// and last cycle's load, W.
    pub channels: [(u8, u32); 3],
    /// The cover is locked and a crowbar won't open it.
    pub cover_locked: bool,
    /// The viewer is close enough to press the buttons.
    pub controls: bool,
}

/// One air alarm readout: a message id taking `$value`, and whether it is out of range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmReading {
    pub id: String,
    pub value: i32,
    pub alert: bool,
}

/// An open container, as shown to the player carrying it: the items in it, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageView {
    /// The container.
    pub target: NetId,
    pub items: Vec<NetId>,
    /// How many it can hold.
    pub slots: u8,
}

/// What a person wears, as seen by clients: slot → item.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WornView(pub Vec<(Slot, NetId)>);

impl WornView {
    pub fn get(&self, slot: Slot) -> Option<NetId> {
        self.0.iter().find(|(s, _)| *s == slot).map(|(_, id)| *id)
    }
}

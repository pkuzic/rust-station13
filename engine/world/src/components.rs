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

/// Blocks movement into its tile while present (BYOND `density`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dense;

/// Blocks sight through its tile while present (BYOND `opacity`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opaque;

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

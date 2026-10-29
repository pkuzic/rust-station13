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

//! ECS components for things placed on the grid.

use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

use crate::{TileMap, TilePos};

/// The tile an entity stands on.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPos(pub TilePos);

/// One deck of the map, held by a single map entity.
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapLayer(pub TileMap);

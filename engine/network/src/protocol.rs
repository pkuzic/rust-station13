//! Everything that crosses the wire: replicated components, messages and channels.
//! Client and server must register the same protocol.

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use lightyear::prelude::*;
use rs13_world::{Dir, GridPos, MapLayer};
use serde::{Deserialize, Serialize};

/// A player-controlled mob. Replicated to every client.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// Netcode client id of the controlling player.
    pub client_id: u64,
}

/// Client → server: the movement the player is currently asking for. Sent when it
/// changes; the server keeps applying the last one received.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveInput {
    pub dir: Option<Dir>,
    pub run: bool,
}

/// Reliable, ordered channel for player intentions.
pub struct IntentChannel;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<IntentChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        app.register_message::<MoveInput>()
            .add_direction(NetworkDirection::ClientToServer);

        app.component::<MapLayer>().replicate();
        app.component::<GridPos>().replicate();
        app.component::<Player>().replicate();
    }
}

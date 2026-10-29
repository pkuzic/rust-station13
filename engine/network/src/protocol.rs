//! Everything that crosses the wire: replicated components, messages and channels.
//! Client and server must register the same protocol.

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use lightyear::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, DisplayName, GridPos, HandsView, Intent, MapLayer, MobStat, NetId};
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

/// Client → server: a player intention other than movement. Entities are named by
/// [`NetId`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ActionInput {
    /// Click with the active hand.
    UseOn(NetId),
    Drop,
    SwapHands,
    Examine(NetId),
    /// Say something out loud. The server cleans it up and limits its length.
    Say(String),
    /// Choose a character name. Accepted once per session, if valid.
    SetName(String),
    SetIntent(Intent),
}

/// Server → the controlling client: its own health, state and intent. Sent when any of
/// them changes. Other players' health is not sent.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusUpdate {
    pub health: i32,
    pub stat: MobStat,
    pub intent: Intent,
}

/// Server → one client: a message for the player, formatted by the client in its
/// own language.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Notice(pub LocMsg);

/// Reliable, ordered channel for player intentions.
pub struct IntentChannel;

/// Reliable, ordered channel for messages shown to the player.
pub struct NoticeChannel;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<IntentChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        app.add_channel::<NoticeChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<MoveInput>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<ActionInput>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<Notice>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<StatusUpdate>()
            .add_direction(NetworkDirection::ServerToClient);

        app.component::<MapLayer>().replicate();
        app.component::<GridPos>().replicate();
        app.component::<Player>().replicate();
        app.component::<NetId>().replicate();
        app.component::<Appearance>().replicate();
        app.component::<HandsView>().replicate();
        app.component::<DisplayName>().replicate();
    }
}

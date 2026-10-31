//! Everything that crosses the wire: replicated components, messages and channels.
//! Client and server must register the same protocol.

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use lightyear::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    AlarmPanelView, ApcPanelView, Appearance, AtmosView, Dir, DisplayName, EnvAlert, Facing,
    FreezerPanelView, GridPos, HandsView, Hidden, Intent, LaserPanelView, LightSource, MapLayer, MobStat,
    NetId, Opaque, PixelOffset, PowerMonitorView, Slot, SmesPanelView, StorageView, TilePos, WirePanelView,
    WornView,
};
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
    /// Put the active-hand item on in this slot.
    Equip(Slot),
    /// Take off what is in this slot, into the empty active hand.
    Unequip(Slot),
    /// Connect the mask to a gas tank, or disconnect it.
    ToggleInternals,
    /// On an open wire panel: cut or mend a wire (wirecutters), or pulse it (multitool).
    /// `colour` is the wire's index in the panel.
    Wire {
        target: NetId,
        colour: u8,
        pulse: bool,
    },
    /// Close the open machine window (wire panel or air alarm).
    ClosePanel,
    /// A button on an open air alarm window.
    Alarm {
        target: NetId,
        command: AlarmCommand,
    },
    /// A button on an open APC window.
    Apc {
        target: NetId,
        command: ApcCommand,
    },
    /// A button on an open laser control computer.
    Laser {
        target: NetId,
        command: LaserCommand,
    },
    /// A button on an open freezer.
    Freezer {
        target: NetId,
        command: FreezerCommand,
    },
    /// A button on an open SMES.
    Smes {
        target: NetId,
        command: SmesCommand,
    },
    /// Close the open container.
    CloseStorage,
    /// Click on a tile with nothing on it to click: lay cable from a coil.
    UseOnTile(TilePos),
}

/// Air alarm window buttons.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlarmCommand {
    /// Turn the area's emergency seal on or off.
    Seal(bool),
    /// Turn panic fill on the area's vents on or off.
    PanicFill(bool),
}

/// APC window buttons (Luna's `apc` Topic links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApcCommand {
    /// Flip the main breaker.
    Breaker,
    /// Set a channel (0 equipment, 1 lighting, 2 environment) off, on or automatic.
    Channel { channel: u8, mode: ApcChannelSetting },
    /// Turn automatic cell charging on or off.
    ChargeMode,
    /// Engage or disengage the cover lock.
    CoverLock,
}

/// Laser control computer buttons (Luna's `lascomp` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaserCommand {
    /// Change every laser's power by this much (kept within 1–3000).
    Power(i32),
    /// Turn every laser on or off.
    Toggle,
}

/// Freezer buttons (Luna's `freezer` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreezerCommand {
    /// Switch it on or off.
    Toggle,
    /// Change the temperature it cools to by this many kelvin.
    Setting(i32),
}

/// SMES window buttons (Luna's `smes` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmesCommand {
    /// Automatic charging on or off.
    ChargeMode,
    /// Output on or off.
    Online,
    /// Change the input level.
    Input(SmesLevel),
    /// Change the output level.
    Output(SmesLevel),
}

/// How a SMES level button changes it: to nothing, to the most, or by an amount (W).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmesLevel {
    Min,
    Max,
    Change(i32),
}

/// What a channel button sets.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApcChannelSetting {
    Off,
    On,
    Auto,
}

/// Server → the controlling client: its own health, state and intent. Sent when any of
/// them changes. Other players' health is not sent.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StatusUpdate {
    pub health: i32,
    pub stat: MobStat,
    pub intent: Intent,
    /// Pressure where the player stands, tenths of a kPa.
    pub pressure_dkpa: u32,
    pub alerts: Vec<EnvAlert>,
    /// Breathing from a tank.
    pub internals: bool,
    /// Body temperature, kelvin.
    pub body_temperature: u16,
}

/// Server → one client: a message for the player, formatted by the client in its
/// own language.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Notice(pub LocMsg);

/// Server → one client: the wire panel it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WirePanel(pub Option<WirePanelView>);

/// Server → one client: the air alarm window it has open, or `None` to close it. Sent when
/// it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AlarmPanel(pub Option<AlarmPanelView>);

/// Server → one client: the APC window it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApcPanel(pub Option<ApcPanelView>);

/// Server → one client: the power monitor it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PowerMonitorPanel(pub Option<PowerMonitorView>);

/// Server → one client: the laser control computer it has open, or `None` to close it.
/// Sent when it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LaserPanel(pub Option<LaserPanelView>);

/// Server → one client: the freezer it has open, or `None` to close it. Sent when it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FreezerPanel(pub Option<FreezerPanelView>);

/// Server → one client: the SMES it has open, or `None` to close it. Sent when it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SmesPanel(pub Option<SmesPanelView>);

/// Server → one client: the container it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoragePanel(pub Option<StorageView>);

/// Server → one client: a sound its player hears. The server has already decided that the
/// player hears it and how loud, after distance, walls, doors and air; the client only pans
/// it by where it came from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlaySound {
    /// Sound id, e.g. `airlock`. The client picks the file.
    pub sound: String,
    /// The tile it came from.
    pub at: TilePos,
    /// Loudness for this player, 1–100.
    pub volume: u8,
}

/// Reliable, ordered channel for player intentions.
pub struct IntentChannel;

/// Reliable, ordered channel for messages shown to the player.
pub struct NoticeChannel;

/// Unreliable channel for sounds: a late sound is worse than a lost one.
pub struct SoundChannel;

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

        app.add_channel::<SoundChannel>(ChannelSettings {
            mode: ChannelMode::UnorderedUnreliable,
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
        app.register_message::<WirePanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<AlarmPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<StoragePanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<ApcPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PowerMonitorPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<LaserPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<FreezerPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<SmesPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PlaySound>()
            .add_direction(NetworkDirection::ServerToClient);

        app.component::<MapLayer>().replicate();
        app.component::<GridPos>().replicate();
        app.component::<Player>().replicate();
        app.component::<NetId>().replicate();
        app.component::<Appearance>().replicate();
        app.component::<PixelOffset>().replicate();
        app.component::<Facing>().replicate();
        app.component::<Hidden>().replicate();
        app.component::<HandsView>().replicate();
        app.component::<WornView>().replicate();
        app.component::<DisplayName>().replicate();
        app.component::<AtmosView>().replicate();
        app.component::<LightSource>().replicate();
        app.component::<Opaque>().replicate();
    }
}

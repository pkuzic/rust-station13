//! Everything that crosses the wire: replicated components, messages and channels.
//! Client and server must register the same protocol.

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use lightyear::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    AlarmPanelView, ApcPanelView, Appearance, ArcadePanelView, AtmosView, BrigPanelView, CameraPanelView,
    CameraView, ClipboardView, Dir, DisplayName, EnvAlert, Facing, FreezerPanelView, GasPanelView, Ghost,
    GridPos, HandsView, HeaterPanelView, Hidden, Intent, LaserPanelView, LightSource, MapLayer, MobStat,
    NetId, Opaque, PaperView, PickListView, PixelOffset, PowerMonitorView, Silhouette, Slot, SmesPanelView,
    StorageView, TilePos, VendingPanelView, WirePanelView, WornView,
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
    /// A ghost starts over as a new character (Luna `abandon_mob`).
    Respawn,
    Drop,
    SwapHands,
    /// Turn to face a way without stepping (Ctrl + a direction).
    Face(Dir),
    /// One of an object's verbs from the right-click menu, by name (`toggle-open`). The
    /// server decides whether it applies.
    Verb {
        target: NetId,
        verb: String,
    },
    /// Start pulling something within a tile (Ctrl-click).
    Pull(NetId),
    /// Let go of what is being pulled (the HUD pull button).
    StopPulling,
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
    /// The whole new text of a sheet of paper, from its text box.
    WritePaper {
        target: NetId,
        text: String,
    },
    /// A sheet's new label, from its label box.
    LabelPaper {
        target: NetId,
        label: String,
    },
    /// A button on an open brig door timer.
    Brig {
        target: NetId,
        command: BrigCommand,
    },
    /// A button on an open arcade machine.
    Arcade {
        target: NetId,
        command: ArcadeCommand,
    },
    /// A button on an open space heater.
    Heater {
        target: NetId,
        command: HeaterCommand,
    },
    /// A line picked in an open list window, from 0.
    Pick {
        target: NetId,
        line: u16,
    },
    /// A button on an open clipboard.
    Clipboard {
        target: NetId,
        command: ClipboardCommand,
    },
    /// A camera picked on an open security console (its line, from 0), or `None` to stop
    /// watching.
    Camera {
        target: NetId,
        line: Option<u16>,
    },
    /// A product's Vend button on an open vending machine: its line, from 0.
    Vend {
        target: NetId,
        line: u8,
    },
    /// A button on an open canister, portable pump or scrubber.
    Gas {
        target: NetId,
        command: GasCommand,
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

/// Canister, portable pump and scrubber window buttons (Luna's `canister`, `pump` and
/// `scrubber` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasCommand {
    /// A canister's release valve, or a pump's or scrubber's power.
    Toggle,
    /// A pump's direction.
    Direction,
    /// Change the release pressure or target (kPa), or the rate (L), by this much.
    Adjust(i32),
    /// Take the held tank out.
    RemoveTank,
}

/// Door timer window buttons (Luna's `door_timer` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrigCommand {
    Toggle,
    /// Seconds to add, or to take off.
    Adjust(i16),
    Flash,
}

/// Arcade window buttons (Luna's `arcade` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcadeCommand {
    Attack,
    Heal,
    Recharge,
    NewGame,
}

/// Space heater window buttons (Luna's `spaceheater` links).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaterCommand {
    /// Take the cell out, or put the one in hand in.
    Cell,
    /// Set it 5 °C warmer (`true`) or cooler.
    Warmer(bool),
}

/// Clipboard window buttons (Luna's `clipboard` links); sheets by line, the top one 0.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardCommand {
    /// Take the pen out, or put the one in hand in.
    Pen,
    Write(u8),
    Remove(u8),
    Read(u8),
    Top(u8),
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
    /// Goes up each time the player is dazzled by a flash: their view whites out.
    pub dazzled: u8,
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
    /// Pulling something (the HUD pull button lights up).
    pub pulling: bool,
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

/// Server → one client: a sheet of paper to read, write on or label.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PaperPanel(pub PaperView);

/// Server → one client: the clipboard it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClipboardPanel(pub Option<ClipboardView>);

/// Server → one client: the space heater it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HeaterPanel(pub Option<HeaterPanelView>);

/// Server → one client: the door timer it has open, or `None` to close it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BrigPanel(pub Option<BrigPanelView>);

/// Server → one client: the arcade game it has open, or `None` to close it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArcadePanel(pub Option<ArcadePanelView>);

/// Server → one client: the list window it has open, or `None` to close it. Sent when it
/// changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PickListPanel(pub Option<PickListView>);

/// Server → one client: the security console it has open, or `None` to close it. Sent
/// when it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CameraPanel(pub Option<CameraPanelView>);

/// Server → one client: the vending machine it has open, or `None` to close it. Sent when
/// it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct VendingPanel(pub Option<VendingPanelView>);

/// Server → one client: the canister, portable pump or scrubber it has open, or `None` to
/// close it. Sent when it changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GasPanel(pub Option<GasPanelView>);

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
        app.register_message::<GasPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<VendingPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<CameraPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PaperPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<ClipboardPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PickListPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<HeaterPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<BrigPanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<ArcadePanel>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<PlaySound>()
            .add_direction(NetworkDirection::ServerToClient);

        app.component::<MapLayer>().replicate();
        app.component::<GridPos>().replicate();
        app.component::<CameraView>().replicate();
        app.component::<Player>().replicate();
        app.component::<NetId>().replicate();
        app.component::<Appearance>().replicate();
        app.component::<PixelOffset>().replicate();
        app.component::<Facing>().replicate();
        app.component::<Hidden>().replicate();
        app.component::<HandsView>().replicate();
        app.component::<WornView>().replicate();
        app.component::<Ghost>().replicate();
        app.component::<DisplayName>().replicate();
        app.component::<AtmosView>().replicate();
        app.component::<LightSource>().replicate();
        app.component::<Opaque>().replicate();
        app.component::<Silhouette>().replicate();
    }
}

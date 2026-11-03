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

/// Which way a placed thing faces, as a DMI direction index (0 S, 1 N, 2 E, 3 W, 4 SE, 5 SW,
/// 6 NE, 7 NW): the client draws that direction of its sprite (BYOND `dir`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facing(pub u8);

impl Facing {
    pub fn of(dir: crate::Dir) -> Self {
        use crate::Dir::*;
        Self(match dir {
            South => 0,
            North => 1,
            East => 2,
            West => 3,
            SouthEast => 4,
            SouthWest => 5,
            NorthEast => 6,
            NorthWest => 7,
        })
    }

    /// The direction this facing points.
    pub fn dir(self) -> crate::Dir {
        use crate::Dir::*;
        match self.0 {
            1 => North,
            2 => East,
            3 => West,
            4 => SouthEast,
            5 => SouthWest,
            6 => NorthEast,
            7 => NorthWest,
            _ => South,
        }
    }
}

/// Drawn this many pixels east and north of its tile (BYOND `pixel_x`, `pixel_y`): how a map
/// hangs a device on the wall beside the floor tile it belongs to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelOffset(pub [i8; 2]);

/// Seen by no one and out of reach while present, such as a pipe or cable under floor
/// tiles (BYOND `invisibility = 101`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hidden;

/// Blocks movement into its tile while present (BYOND `density`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dense;

/// A person standing in the dark, as the server sends them to everyone else: where they
/// are, which way they face and their body's pose, but not who they are, what they wear
/// or hold. Drawn as a black shape. Clicking it acts on the person.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Silhouette;

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
    /// Left the body: a player walking on as a ghost.
    Ghost,
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

/// The air of every tile of the deck, row-major like the map: pressure in tenths of a kPa,
/// for the atmos debug overlay, and the gases you can see (plasma and sleeping gas) in
/// tenths of a mole, saturating at 25.5 mol, for the gas overlays.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AtmosView {
    pub pressure_dkpa: Vec<u16>,
    pub plasma_dmol: Vec<u8>,
    pub n2o_dmol: Vec<u8>,
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
    /// Handcuffs on your wrists: nobody puts these on themselves (`security.md`).
    Cuffs,
}

impl Slot {
    pub const ALL: [Slot; 12] = [
        Slot::Cuffs,
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
            Slot::Cuffs => "cuffs",
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

/// An open power monitoring computer, as shown to the player using it (Luna's `powcomp`
/// window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerMonitorView {
    /// The computer.
    pub target: NetId,
    /// Power the grid offered last cycle, W.
    pub supply: u32,
    /// Power drawn from it, W.
    pub load: u32,
    /// Every APC, by area id.
    pub apcs: Vec<PowerMonitorRow>,
}

/// One APC on the power monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerMonitorRow {
    /// Area id: the client shows the `area-<id>` message.
    pub area: String,
    /// Equipment, lighting and environment modes (Luna 0 off, 1 auto-off, 2 on, 3 auto-on).
    pub channels: [u8; 3],
    /// Last cycle's load, W.
    pub load: u32,
    /// Cell charge, percent; `None` with no cell.
    pub cell_percent: Option<u8>,
    /// 0 not charging, 1 charging, 2 full.
    pub charging: u8,
}

/// An open laser control computer, as shown to the player using it (Luna's `lascomp`
/// window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaserPanelView {
    /// The computer.
    pub target: NetId,
    /// How many lasers it controls; none means "No laser found".
    pub lasers: u8,
    /// The first laser's power level (1–3000) and whether it is firing.
    pub power: u32,
    pub online: bool,
}

/// An open freezer, as shown to the player using it (Luna's `freezer` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezerPanelView {
    /// The freezer.
    pub target: NetId,
    pub on: bool,
    /// Its gas: temperature in tenths of a kelvin, pressure in tenths of a kPa.
    pub gas_dk: u32,
    pub pressure_dkpa: u32,
    /// The temperature it cools to, K.
    pub setting: u32,
}

/// On a person watching through a security camera: the camera's tile. They see from there
/// (the server sends what it sees, and the client centres the view on it).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraView(pub TilePos);

/// One camera in a security console's list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraRow {
    /// Its name tag (Luna `c_tag`).
    pub tag: String,
    /// It works.
    pub on: bool,
}

/// An open security camera console, as shown to the player using it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraPanelView {
    /// The console.
    pub target: NetId,
    pub cameras: Vec<CameraRow>,
    /// The line of the camera it shows, if any.
    pub current: Option<u16>,
}

/// What a paper window is for (Luna: reading it, its text box, its label box).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperMode {
    Read,
    Write,
    Label,
}

/// A sheet of paper to read, write on or label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperView {
    /// The sheet.
    pub target: NetId,
    /// Its name (Luna: "Paper", or "paper- 'label'").
    pub title: String,
    /// What is written on it, with Luna's [b], [i] and [u] tags.
    pub text: String,
    pub mode: PaperMode,
}

/// An open clipboard, as shown to the player carrying it (Luna's `clipboard` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardView {
    /// The clipboard.
    pub target: NetId,
    pub has_pen: bool,
    /// Its sheets' names, the top one first.
    pub papers: Vec<String>,
}

/// An open chem dispenser's window, as shown to the player (Luna's `chem_dispenser` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChemPanelView {
    /// The machine.
    pub target: NetId,
    pub energy: u16,
    /// Units it gives at a time.
    pub amount: u8,
    /// Its beaker's contents and size, if it has one.
    pub beaker: Option<(u16, u16)>,
    /// What it can give, by reagent id.
    pub reagents: Vec<String>,
}

/// An open identification computer's window, as shown to the player (Luna's `computer/card`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdComputerView {
    /// The computer.
    pub target: NetId,
    pub has_card: bool,
    /// Whose the card in it is, and its job, by id.
    pub owner: Option<String>,
    pub job: Option<String>,
    /// Every job, by id.
    pub jobs: Vec<String>,
    /// The crew manifest instead (Luna's `mode`): each person's name and job, by id.
    pub manifest: Option<Vec<(String, String)>>,
    /// A copy of it is on its way out.
    pub printing: bool,
}

/// One person's line in a records window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRow {
    pub name: String,
    /// Their job, by id.
    pub job: String,
    /// Their criminal status or condition, by id.
    pub status: String,
}

/// An open records computer's window, as shown to the player (Luna's `secure_data`, `med_data`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordsPanelView {
    /// The computer.
    pub target: NetId,
    pub medical: bool,
    pub rows: Vec<RecordRow>,
}

/// An open communications console's window, as shown to the player (Luna's `comm_console`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommsPanelView {
    /// The console.
    pub target: NetId,
    /// Seconds until the escape pods launch, if they've been called.
    pub launch_in: Option<u32>,
    /// They've gone.
    pub gone: bool,
}

/// The lobby, as one player sees it (Luna's `new_player` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyView {
    /// Seconds until the round starts, or `None` once it has.
    pub starts_in: Option<u32>,
    pub ready: bool,
    pub ready_count: u16,
    pub players: u16,
    /// The name they've asked for, if any.
    pub name: Option<String>,
    /// The job they'd like, by id.
    pub preference: Option<String>,
    /// Every job, by id, and how many posts are free.
    pub jobs: Vec<(String, u16)>,
}

/// The job window a new arrival sees (Luna's job choice).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobPanelView {
    /// The job they have, by id.
    pub current: String,
    /// Every job, by id, and how many posts are free.
    pub rows: Vec<(String, u16)>,
}

/// An open body scanner console's window, as shown to the player (Luna's `scanconsole` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyScanView {
    /// The console.
    pub target: NetId,
    /// Who is in its scanner.
    pub occupant: Option<String>,
    pub stat: MobStat,
    pub health: i16,
    pub oxy: i16,
    pub tox: i16,
    /// The damaged parts, by noun id: brute and burn.
    pub parts: Vec<(String, i16, i16)>,
}

/// One line of an autolathe's window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutolatheRow {
    pub item: String,
    pub metal: i32,
    pub glass: i32,
    pub can_make: bool,
}

/// An open autolathe's window, as shown to the player (Luna's `autolathe` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutolathePanelView {
    /// The machine.
    pub target: NetId,
    pub metal: i32,
    pub glass: i32,
    /// Whether it's making something now.
    pub busy: bool,
    pub rows: Vec<AutolatheRow>,
}

/// An open CheMaster's window, as shown to the player (Luna's `chemmaster` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChemMasterPanelView {
    /// The machine.
    pub target: NetId,
    /// What's in its beaker, by reagent id, if it has one.
    pub beaker: Option<Vec<(String, u16)>>,
    /// What's in its buffer.
    pub buffer: Vec<(String, u16)>,
}

/// An open sleeper console's window, as shown to the player (Luna's `sleep_console` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleeperPanelView {
    /// The console.
    pub target: NetId,
    /// Who is in its sleeper.
    pub occupant: Option<String>,
    pub stat: MobStat,
    pub health: i16,
    pub oxy: i16,
    pub tox: i16,
    /// What it can inject, by reagent id, and how much of each is in them.
    pub reagents: Vec<(String, u16)>,
}

/// An open brig door timer's window, as shown to the player (Luna's `door_timer` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrigPanelView {
    /// The timer.
    pub target: NetId,
    pub timing: bool,
    /// Seconds left, or the sentence set.
    pub seconds: i32,
    /// There's a flasher in the cell, and whether it is resting.
    pub has_flash: bool,
    pub flash_charging: bool,
}

/// An open arcade machine's window, as shown to the player (Luna's `arcade` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcadePanelView {
    /// The machine.
    pub target: NetId,
    /// Who the villain is: its title and name, as selectors in the locale.
    pub title: String,
    pub villain: String,
    /// What just happened: a message id, and its numbers.
    pub message: String,
    pub value: i32,
    pub cost: i32,
    pub player_hp: i32,
    pub player_mp: i32,
    pub enemy_hp: i32,
    pub game_over: bool,
}

/// An open space heater's window, as shown to the player (Luna's `spaceheater` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaterPanelView {
    /// The heater.
    pub target: NetId,
    pub has_cell: bool,
    /// Its cell's charge, %.
    pub charge_percent: u8,
    /// The temperature it heats to, °C.
    pub setting: i16,
}

/// A window listing things to pick one from, such as the sheets in a filing cabinet
/// (Luna's `filingcabinet` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickListView {
    /// What holds them.
    pub target: NetId,
    /// Its noun id, for the window's title.
    pub title: String,
    /// Each thing's name, in the order listed.
    pub rows: Vec<String>,
}

/// One product line of a vending machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendingRow {
    /// The product's prototype id (its noun).
    pub item: String,
    /// How many are left.
    pub amount: u16,
}

/// An open vending machine, as shown to the player using it (Luna's `vending` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendingPanelView {
    /// The machine.
    pub target: NetId,
    /// Its prototype id (its noun), for the title.
    pub machine: String,
    pub products: Vec<VendingRow>,
    /// Not busy handing something out.
    pub ready: bool,
}

/// Which kind of portable gas machine a [`GasPanelView`] shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GasMachine {
    Canister,
    Pump,
    Scrubber,
}

/// An open canister, portable pump or scrubber, as shown to the player using it (Luna's
/// `canister`, `pump` and `scrubber` windows).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPanelView {
    /// The machine.
    pub target: NetId,
    pub kind: GasMachine,
    /// Its own gas, tenths of a kPa.
    pub pressure_dkpa: u32,
    /// Fastened to a connector port.
    pub connected: bool,
    /// A canister's release valve is open, or a pump or scrubber is on.
    pub on: bool,
    /// A pump releases its tank (rather than siphoning into it).
    pub out: bool,
    /// A canister's release pressure or a pump's target, kPa; a scrubber's rate, L.
    pub setting: u32,
    /// The gas tank held in it, if any: its pressure, tenths of a kPa.
    pub tank_dkpa: Option<u32>,
}

/// An open SMES, as shown to the player using it (Luna's `smes` window).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmesPanelView {
    /// The SMES.
    pub target: NetId,
    /// Stored charge, percent.
    pub charge_percent: u8,
    pub charging: bool,
    pub charge_auto: bool,
    /// Input and output levels, W.
    pub input: u32,
    pub output: u32,
    pub online: bool,
    /// Power it put out last cycle, W.
    pub load: u32,
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

/// A dead player walking on, drawn see-through. The living are never sent one. It carries
/// what the body wore when they died, as those clothes may since have gone elsewhere.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ghost {
    pub worn: Vec<(Slot, Appearance)>,
}

/// What a person wears, as seen by clients: slot → item.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WornView(pub Vec<(Slot, NetId)>);

impl WornView {
    pub fn get(&self, slot: Slot) -> Option<NetId> {
        self.0.iter().find(|(s, _)| *s == slot).map(|(_, id)| *id)
    }
}

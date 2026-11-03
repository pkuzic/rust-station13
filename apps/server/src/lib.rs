//! The dedicated server as a library, so tests can run it in-process.

mod sight;
mod silhouettes;

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::alarms::{AlarmPanelOut, PendingAlarmOps};
use rs13_classic::apc::{ApcPanelOut, PendingApcOps};
use rs13_classic::arcade::{ArcadeOp, ArcadePanelOut, PendingArcadeOps};
use rs13_classic::autolathe::{LathePanelOut, PendingLatheOps};
use rs13_classic::brig::{BrigOp, BrigPanelOut, PendingBrigOps};
use rs13_classic::cameras::{CameraPanelOut, PendingCameraOps};
use rs13_classic::chem_dispenser::{ChemOp, ChemPanelOut, PendingChemOps};
use rs13_classic::chem_master::{MasterOp, MasterPanelOut, PendingMasterOps};
use rs13_classic::cloning::Arrivals;
use rs13_classic::combat::CombatIntent;
use rs13_classic::environment::{Lungs, Surroundings, alerts};
use rs13_classic::equipment::{EquipAction, PendingEquipment};
use rs13_classic::filing::{PendingPicks, PickListOut};
use rs13_classic::freezers::{FreezerOp, FreezerPanelOut, PendingFreezerOps};
use rs13_classic::gas_panel::{GasOp, GasPanelOut, PendingGasOps};
use rs13_classic::ghosts::Departures;
use rs13_classic::health::Vitals;
use rs13_classic::heaters::{HeaterOp, HeaterPanelOut, PendingHeaterOps};
use rs13_classic::jobs::{JobPanelOut, PendingJobOps};
use rs13_classic::medbay::{PendingSleeperOps, ScanPanelOut, SleeperPanelOut};
use rs13_classic::movement::{Mob, MoveIntent};
use rs13_classic::names::sanitize_name;
use rs13_classic::objects::{spawn_player_mob, spawn_station};
use rs13_classic::paperwork::{ClipboardOut, PaperOp, PaperOut, PendingPaperOps};
use rs13_classic::power_monitor::PowerMonitorOut;
use rs13_classic::records::{PendingRecordOps, RecordsPanelOut};
use rs13_classic::smes_panel::{Level, PendingSmesOps, SmesOp, SmesPanelOut};
use rs13_classic::sound::Earshot;
use rs13_classic::speech::PendingSpeech;
use rs13_classic::storage::StorageOut;
use rs13_classic::supermatter::{LaserOp, LaserPanelOut, PendingLaserOps};
use rs13_classic::vending::{PendingVendOps, VendingPanelOut};
use rs13_classic::wires::{PendingWires, WireOp, WirePanelOut};
use rs13_classic::{ClassicPlugin, maps};
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::server::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{
    ActionInput, AlarmCommand, AlarmPanel, ApcChannelSetting, ApcCommand, ApcPanel, ArcadeCommand,
    ArcadePanel, AutolathePanel, BodyScanPanel, BrigCommand, BrigPanel, CameraPanel, ChemCommand,
    ChemMasterCommand, ChemMasterPanel, ChemPanel, ClipboardCommand, ClipboardPanel, FreezerCommand,
    FreezerPanel, GasCommand, GasPanel, HeaterCommand, HeaterPanel, JobPanel, LaserCommand, LaserPanel,
    MoveInput, Notice, NoticeChannel, PaperPanel, PickListPanel, PlaySound, Player, PowerMonitorPanel,
    RecordsPanel, SleeperPanel, SmesCommand, SmesLevel, SmesPanel, SoundChannel, StatusUpdate, StoragePanel,
    VendingPanel, WirePanel,
};
use rs13_network::server::{ServerNetworkPlugin, start_server};
use rs13_world::{
    AlarmPanelView, ApcPanelView, ArcadePanelView, AutolathePanelView, BodyScanView, BrigPanelView,
    CameraPanelView, ChemMasterPanelView, ChemPanelView, ClipboardView, DisplayName, FreezerPanelView,
    GasPanelView, Ghost, GridPos, HeaterPanelView, Intent, JobPanelView, LaserPanelView, MapLayer, MobStat,
    NetId, PaperView, PickListView, PowerMonitorView, RecordsPanelView, SleeperPanelView, Slot,
    SmesPanelView, StorageView, TileMap, TilePos, VendingPanelView, WirePanelView,
};

#[derive(Resource)]
struct Port(u16);

/// The map to load, in the RON map format.
#[derive(Resource)]
/// The station's decks as map text, bottom deck first: deck `i` is at `z = i`.
struct MapText(Vec<&'static str>);

/// A ghost's player asking to start over, from their link.
#[derive(Message, Debug, Clone, Copy)]
struct RespawnRequest {
    link: Entity,
    ghost: Entity,
}

/// What a ghost's status says its body temperature is: neither hot nor cold on the screen.
const GHOST_BODY_TEMPERATURE: u16 = 310;

/// On a client's link entity: the mob that client controls.
#[derive(Component)]
struct Controls(Entity);

/// On a client's link entity: the status last sent to it.
#[derive(Component, Default)]
struct SentStatus(Option<StatusUpdate>);

/// The wire panel, air alarm window and open container last sent to this client.
#[derive(Component, Default)]
struct SentPanel(
    Option<WirePanelView>,
    Option<AlarmPanelView>,
    Option<StorageView>,
    Option<ApcPanelView>,
    Option<PowerMonitorView>,
    Option<LaserPanelView>,
    Option<FreezerPanelView>,
    Option<SmesPanelView>,
    Option<GasPanelView>,
    Option<VendingPanelView>,
    Option<CameraPanelView>,
    Option<ClipboardView>,
    Option<PickListView>,
    Option<HeaterPanelView>,
    Option<ArcadePanelView>,
    Option<BrigPanelView>,
    Option<ChemPanelView>,
    Option<SleeperPanelView>,
    Option<ChemMasterPanelView>,
    Option<AutolathePanelView>,
    Option<BodyScanView>,
    Option<JobPanelView>,
    Option<RecordsPanelView>,
);

/// On a mob whose player has already chosen a name this session.
#[derive(Component)]
struct NameChosen;

/// A server listening on UDP `port` with the engineering chamber, ready to `run()`.
/// Logging is left to the caller.
pub fn server_app(port: u16) -> App {
    server_app_with_map(port, maps::ENGINEERING_CHAMBER)
}

/// A server listening on UDP `port` with the given map (RON map format).
pub fn server_app_with_map(port: u16, map: &'static str) -> App {
    server_app_with_station(port, vec![map])
}

/// A server for a station of several decks, bottom deck first.
pub fn server_app_with_station(port: u16, decks: Vec<&'static str>) -> App {
    let mut app = App::new();
    app.add_plugins((
        // Wake often enough to service the network; the game itself runs on fixed ticks.
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(2))),
        StatesPlugin,
    ))
    .add_plugins((ServerNetworkPlugin, ClassicPlugin))
    .insert_resource(Port(port))
    .insert_resource(MapText(decks))
    .add_systems(Startup, (load_map, listen))
    .add_message::<RespawnRequest>()
    .add_observer(replicate_net_entities)
    .add_observer(spawn_player)
    // Lightyear drops messages not read in the frame they arrive, and a frame may
    // have no fixed tick, so read inputs every frame; the game applies them on ticks.
    .add_systems(
        Update,
        (
            apply_move_inputs,
            queue_actions,
            respawn_ghosts.after(queue_actions),
            send_notices,
            send_sounds,
            send_status,
            send_panels,
            send_paper,
        ),
    )
    // What each client is sent follows what its mob sees: once a tick, after the tick, and
    // for a client that has just joined, before anything is sent to it.
    .add_systems(
        FixedPostUpdate,
        (
            follow_to_ghosts,
            follow_to_clones,
            ApplyDeferred,
            silhouettes::update_silhouettes,
            sight::update_sight::<()>,
        )
            .chain(),
    )
    .add_systems(
        PostUpdate,
        sight::update_sight::<Without<sight::OutOfSight>>.before(ReplicationSystems::Send),
    );
    app
}

fn load_map(world: &mut World) {
    let texts = world.resource::<MapText>().0.clone();
    let decks: Vec<TileMap> = texts
        .iter()
        .enumerate()
        .map(|(z, text)| TileMap::from_ron(text, z as i32).expect("map text is valid"))
        .collect();
    for map in &decks {
        info!(
            deck = map.z,
            width = map.width,
            height = map.height,
            spawns = map.spawns.len(),
            "map loaded"
        );
    }
    for proto in spawn_station(world, decks) {
        warn!(proto, "map places an unknown object; skipped");
    }
}

fn listen(mut commands: Commands, port: Res<Port>) {
    start_server(&mut commands, port.0);
    info!(
        port = port.0,
        protocol = rs13_network::PROTOCOL_VERSION,
        tick_rate = rs13_core::TICK_RATE_HZ,
        reference = rs13_classic::REFERENCE_COMMIT,
        "rs13-server listening"
    );
}

/// Everything with a `NetId` is visible to every client.
fn replicate_net_entities(trigger: On<Add<NetId>>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(Replicate::to_clients(NetworkTarget::All));
}

/// Once a client is fully connected, give it a mob on the first free spawn point.
fn spawn_player(
    trigger: On<Add<Connected>>,
    links: Query<&RemoteId, With<ClientOf>>,
    maps: Query<&MapLayer>,
    occupied: Query<&GridPos, With<Mob>>,
    mut commands: Commands,
) {
    let link = trigger.entity;
    let Ok(remote) = links.get(link) else { return };
    let client_id = remote.0.to_bits();

    let taken: Vec<_> = occupied.iter().map(|p| p.0).collect();
    let starts: Vec<_> = maps.iter().flat_map(|m| m.0.spawns.iter().copied()).collect();

    commands.queue(move |world: &mut World| {
        let Some(spawn) = rs13_classic::jobs::pick_start(world, &starts, &taken) else {
            warn!(client_id, "no free spawn point; player left without a mob");
            return;
        };
        let mob = spawn_controlled_mob(world, spawn, client_id, link);
        if let Ok(mut link) = world.get_entity_mut(link) {
            link.insert((Controls(mob), SentStatus::default(), SentPanel::default()));
        }
        info!(client_id, x = spawn.x, y = spawn.y, "player joined");
    });
}

/// A new character at `at`, controlled by this client.
fn spawn_controlled_mob(world: &mut World, at: TilePos, client_id: u64, link: Entity) -> Entity {
    let mob = spawn_player_mob(world, at);
    rs13_classic::jobs::take_up(world, mob, at);
    world.entity_mut(mob).insert((
        Player { client_id },
        ControlledBy {
            owner: link,
            lifetime: Lifetime::SessionBased,
        },
    ));
    mob
}

/// Ghosts who asked to start over become new characters at a free start point, under the
/// name they had (`ghosts.md`).
fn respawn_ghosts(
    mut requests: MessageReader<RespawnRequest>,
    maps: Query<&MapLayer>,
    occupied: Query<&GridPos, With<Mob>>,
    mut ghosts: Query<(&Player, &DisplayName, &mut Outbox), With<Ghost>>,
    mut commands: Commands,
) {
    let mut taken_now: Vec<TilePos> = Vec::new();
    let mut handled: Vec<Entity> = Vec::new();
    for RespawnRequest { link, ghost } in requests.read().copied() {
        // Pressed twice before the first took effect.
        if handled.contains(&ghost) {
            continue;
        }
        handled.push(ghost);
        let Ok((player, name, mut outbox)) = ghosts.get_mut(ghost) else {
            continue;
        };
        let spawn = maps
            .iter()
            .flat_map(|m| m.0.spawns.iter().copied())
            .find(|s| !taken_now.contains(s) && !occupied.iter().any(|p| p.0 == *s));
        let Some(spawn) = spawn else {
            outbox.0.push(LocMsg::new("respawn-no-room"));
            continue;
        };
        taken_now.push(spawn);
        let (client_id, name) = (player.client_id, name.clone());
        commands.queue(move |world: &mut World| {
            let mob = spawn_controlled_mob(world, spawn, client_id, link);
            world.entity_mut(mob).insert((name, NameChosen));
            world.entity_mut(link).insert(Controls(mob));
            world.despawn(ghost);
        });
    }
}

/// A player who has left their body goes on controlling their ghost (`ghosts.md`); a body
/// no player was in leaves no ghost.
fn follow_to_ghosts(
    mut departures: ResMut<Departures>,
    mut links: Query<(Entity, &mut Controls)>,
    players: Query<&Player>,
    mut commands: Commands,
) {
    for (body, ghost) in departures.0.drain(..) {
        let Some((link, mut controls)) = links.iter_mut().find(|(_, c)| c.0 == body) else {
            commands.entity(ghost).despawn();
            continue;
        };
        controls.0 = ghost;
        if let Ok(player) = players.get(body) {
            commands.entity(ghost).insert((
                player.clone(),
                ControlledBy {
                    owner: link,
                    lifetime: Lifetime::SessionBased,
                },
            ));
        }
        // The body stays when they disconnect, and is nobody's now.
        commands.entity(body).try_remove::<(Player, ControlledBy)>();
    }
}

/// A ghost being cloned goes on as the new body in the pod (`cloning.md`).
fn follow_to_clones(
    mut arrivals: ResMut<Arrivals>,
    mut links: Query<(Entity, &mut Controls)>,
    players: Query<&Player>,
    mut commands: Commands,
) {
    for (ghost, body) in arrivals.0.drain(..) {
        if let Some((link, mut controls)) = links.iter_mut().find(|(_, c)| c.0 == ghost) {
            controls.0 = body;
            if let Ok(player) = players.get(ghost) {
                commands.entity(body).insert((
                    player.clone(),
                    ControlledBy {
                        owner: link,
                        lifetime: Lifetime::SessionBased,
                    },
                    NameChosen,
                ));
            }
        }
        commands.entity(ghost).try_despawn();
    }
}

/// Copy the latest movement request from each client onto its mob.
fn apply_move_inputs(
    mut links: Query<(&Controls, &mut MessageReceiver<MoveInput>)>,
    mut intents: Query<&mut MoveIntent>,
) {
    for (controls, mut receiver) in &mut links {
        let Some(input) = receiver.receive().last() else {
            continue;
        };
        if let Ok(mut intent) = intents.get_mut(controls.0) {
            intent.dir = input.dir;
            intent.run = input.run;
        }
    }
}

/// Turn client actions into game actions for the next tick, resolving `NetId`s.
fn queue_actions(
    mut links: Query<(Entity, &Controls, &mut MessageReceiver<ActionInput>)>,
    mut respawns: MessageWriter<RespawnRequest>,
    mut mobs: Query<
        (
            &mut PendingActions,
            &mut PendingSpeech,
            &mut DisplayName,
            &mut Outbox,
            &mut PendingEquipment,
            &mut PendingWires,
            &mut PendingAlarmOps,
            &mut PendingApcOps,
            &mut PendingLaserOps,
            &mut PendingFreezerOps,
            &mut PendingSmesOps,
            &mut PendingGasOps,
            (
                &mut PendingVendOps,
                &mut PendingCameraOps,
                &mut PendingPaperOps,
                &mut PendingPicks,
                &mut PendingHeaterOps,
                &mut PendingArcadeOps,
                &mut PendingBrigOps,
                &mut PendingChemOps,
                &mut PendingSleeperOps,
                &mut PendingMasterOps,
                &mut PendingLatheOps,
                &mut PendingJobOps,
                &mut PendingRecordOps,
            ),
            Has<NameChosen>,
        ),
        Without<Ghost>,
    >,
    ids: Query<(Entity, &NetId, Option<&silhouettes::Shade>)>,
    mut ghosts: Query<(&mut PendingActions, &mut PendingSpeech), With<Ghost>>,
    mut commands: Commands,
) {
    // Clicking someone's stand-in in the dark acts on them (`silhouettes.rs`); examining
    // it tells nothing, as there is nothing to see.
    let find = |id: NetId| ids.iter().find(|(_, n, _)| **n == id);
    let resolve = |id: NetId| find(id).map(|(e, _, shade)| shade.map_or(e, |s| s.0));
    let examined = |id: NetId| find(id).filter(|(_, _, shade)| shade.is_none()).map(|(e, ..)| e);
    for (link, controls, mut receiver) in &mut links {
        let Ok((
            mut pending,
            mut speech,
            mut name,
            mut outbox,
            mut equipment,
            mut wires,
            mut alarm_ops,
            mut apc_ops,
            mut laser_ops,
            mut freezer_ops,
            mut smes_ops,
            mut gas_ops,
            (
                mut vend_ops,
                mut camera_ops,
                mut paper_ops,
                mut picks,
                mut heater_ops,
                mut arcade_ops,
                mut brig_ops,
                mut chem_ops,
                mut sleeper_ops,
                mut master_ops,
                mut lathe_ops,
                mut job_ops,
                mut record_ops,
            ),
            name_chosen,
        )) = mobs.get_mut(controls.0)
        else {
            // A ghost can only talk among the dead and click on things (`ghosts.md`).
            let ghost = ghosts.get_mut(controls.0);
            let Ok((mut pending, mut speech)) = ghost else {
                receiver.receive().for_each(drop);
                continue;
            };
            for input in receiver.receive() {
                let action = match input {
                    ActionInput::UseOn(id) => resolve(id).map(Action::UseOn),
                    ActionInput::Examine(id) => examined(id).map(Action::Examine),
                    ActionInput::Say(text) => {
                        speech.0.push(text);
                        None
                    }
                    ActionInput::Respawn => {
                        respawns.write(RespawnRequest {
                            link,
                            ghost: controls.0,
                        });
                        None
                    }
                    _ => None,
                };
                pending.0.extend(action);
            }
            continue;
        };
        for input in receiver.receive() {
            let action = match input {
                ActionInput::UseOn(id) => resolve(id).map(Action::UseOn),
                ActionInput::Examine(id) => examined(id).map(Action::Examine),
                ActionInput::Pull(id) => resolve(id).map(Action::Pull),
                ActionInput::Verb { target, verb } => rs13_classic::verbs::named(&verb)
                    .and_then(|verb| resolve(target).map(|t| Action::Verb(t, verb))),
                ActionInput::StopPulling => Some(Action::StopPulling),
                ActionInput::Drop => Some(Action::Drop),
                ActionInput::CloseStorage => Some(Action::CloseStorage),
                ActionInput::UseOnTile(tile) => Some(Action::UseOnTile(tile)),
                ActionInput::SwapHands => Some(Action::SwapHands),
                ActionInput::Face(dir) => Some(Action::Face(dir)),
                ActionInput::SetIntent(intent) => Some(Action::SetIntent(intent)),
                // Cuffs come off by being taken off, not as clothes do.
                ActionInput::Unequip(Slot::Cuffs) | ActionInput::Equip(Slot::Cuffs) => None,
                ActionInput::Equip(slot) => {
                    equipment.0.push(EquipAction::Equip(slot));
                    None
                }
                ActionInput::Unequip(slot) => {
                    equipment.0.push(EquipAction::Unequip(slot));
                    None
                }
                ActionInput::ToggleInternals => {
                    equipment.0.push(EquipAction::ToggleInternals);
                    None
                }
                ActionInput::Wire {
                    target,
                    colour,
                    pulse,
                } => {
                    if let Some(door) = resolve(target) {
                        let colour = usize::from(colour);
                        wires.0.push(if pulse {
                            WireOp::Pulse { door, colour }
                        } else {
                            WireOp::CutOrMend { door, colour }
                        });
                    }
                    None
                }
                ActionInput::ClosePanel => {
                    wires.0.push(WireOp::Close);
                    None
                }
                ActionInput::Alarm { target, command } => {
                    if let Some(alarm) = resolve(target) {
                        let command = match command {
                            AlarmCommand::Seal(on) => rs13_classic::alarms::AlarmCommand::Seal(on),
                            AlarmCommand::PanicFill(on) => rs13_classic::alarms::AlarmCommand::PanicFill(on),
                        };
                        alarm_ops.0.push((alarm, command));
                    }
                    None
                }
                ActionInput::Smes { target, command } => {
                    if let Some(unit) = resolve(target) {
                        let level = |l: SmesLevel| match l {
                            SmesLevel::Min => Level::Min,
                            SmesLevel::Max => Level::Max,
                            SmesLevel::Change(d) => Level::Change(f64::from(d)),
                        };
                        smes_ops.0.push((
                            unit,
                            match command {
                                SmesCommand::ChargeMode => SmesOp::ChargeMode,
                                SmesCommand::Online => SmesOp::Online,
                                SmesCommand::Input(l) => SmesOp::Input(level(l)),
                                SmesCommand::Output(l) => SmesOp::Output(level(l)),
                            },
                        ));
                    }
                    None
                }
                ActionInput::WritePaper { target, text } => {
                    if let Some(sheet) = resolve(target) {
                        paper_ops.0.push(PaperOp::Write(sheet, text));
                    }
                    None
                }
                ActionInput::LabelPaper { target, label } => {
                    if let Some(sheet) = resolve(target) {
                        paper_ops.0.push(PaperOp::Label(sheet, label));
                    }
                    None
                }
                ActionInput::Chem { target, command } => {
                    if let Some(machine) = resolve(target) {
                        chem_ops.0.push((
                            machine,
                            match command {
                                ChemCommand::Dispense(line) => ChemOp::Dispense(usize::from(line)),
                                ChemCommand::Amount(amount) => ChemOp::Amount(amount),
                                ChemCommand::Eject => ChemOp::Eject,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Records { target, line } => {
                    if let Some(machine) = resolve(target) {
                        record_ops.0.push((machine, usize::from(line)));
                    }
                    None
                }
                ActionInput::ChooseJob { line } => {
                    job_ops.0.push(line.map(usize::from));
                    None
                }
                ActionInput::Autolathe { target, line } => {
                    if let Some(machine) = resolve(target) {
                        lathe_ops.0.push((machine, usize::from(line)));
                    }
                    None
                }
                ActionInput::ChemMaster { target, command } => {
                    if let Some(machine) = resolve(target) {
                        master_ops.0.push((
                            machine,
                            match command {
                                ChemMasterCommand::Add(line) => MasterOp::Add(usize::from(line)),
                                ChemMasterCommand::Remove(line) => MasterOp::Remove(usize::from(line)),
                                ChemMasterCommand::Pill => MasterOp::Pill,
                                ChemMasterCommand::Bottle => MasterOp::Bottle,
                                ChemMasterCommand::Eject => MasterOp::Eject,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Sleeper { target, line } => {
                    if let Some(console) = resolve(target) {
                        sleeper_ops.0.push((console, usize::from(line)));
                    }
                    None
                }
                ActionInput::Brig { target, command } => {
                    if let Some(machine) = resolve(target) {
                        brig_ops.0.push((
                            machine,
                            match command {
                                BrigCommand::Toggle => BrigOp::Toggle,
                                BrigCommand::Adjust(by) => BrigOp::Adjust(i32::from(by)),
                                BrigCommand::Flash => BrigOp::Flash,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Arcade { target, command } => {
                    if let Some(machine) = resolve(target) {
                        arcade_ops.0.push((
                            machine,
                            match command {
                                ArcadeCommand::Attack => ArcadeOp::Attack,
                                ArcadeCommand::Heal => ArcadeOp::Heal,
                                ArcadeCommand::Recharge => ArcadeOp::Recharge,
                                ArcadeCommand::NewGame => ArcadeOp::NewGame,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Heater { target, command } => {
                    if let Some(heater) = resolve(target) {
                        heater_ops.0.push((
                            heater,
                            match command {
                                HeaterCommand::Cell => HeaterOp::Cell,
                                HeaterCommand::Warmer(true) => HeaterOp::Setting(1),
                                HeaterCommand::Warmer(false) => HeaterOp::Setting(-1),
                            },
                        ));
                    }
                    None
                }
                ActionInput::Pick { target, line } => {
                    if let Some(holder) = resolve(target) {
                        picks.0.push((holder, usize::from(line)));
                    }
                    None
                }
                ActionInput::Clipboard { target, command } => {
                    if let Some(board) = resolve(target) {
                        paper_ops.0.push(match command {
                            ClipboardCommand::Pen => PaperOp::Pen(board),
                            ClipboardCommand::Write(n) => PaperOp::WriteSheet(board, usize::from(n)),
                            ClipboardCommand::Remove(n) => PaperOp::Remove(board, usize::from(n)),
                            ClipboardCommand::Read(n) => PaperOp::Read(board, usize::from(n)),
                            ClipboardCommand::Top(n) => PaperOp::Top(board, usize::from(n)),
                        });
                    }
                    None
                }
                ActionInput::Camera { target, line } => {
                    if let Some(console) = resolve(target) {
                        camera_ops.0.push((console, line.map(usize::from)));
                    }
                    None
                }
                ActionInput::Vend { target, line } => {
                    if let Some(machine) = resolve(target) {
                        vend_ops.0.push((machine, usize::from(line)));
                    }
                    None
                }
                ActionInput::Gas { target, command } => {
                    if let Some(machine) = resolve(target) {
                        gas_ops.0.push((
                            machine,
                            match command {
                                GasCommand::Toggle => GasOp::Toggle,
                                GasCommand::Direction => GasOp::Direction,
                                GasCommand::Adjust(by) => GasOp::Adjust(f64::from(by)),
                                GasCommand::RemoveTank => GasOp::RemoveTank,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Freezer { target, command } => {
                    if let Some(freezer) = resolve(target) {
                        freezer_ops.0.push((
                            freezer,
                            match command {
                                FreezerCommand::Toggle => FreezerOp::Toggle,
                                FreezerCommand::Setting(delta) => FreezerOp::Setting(f64::from(delta)),
                            },
                        ));
                    }
                    None
                }
                ActionInput::Laser { target, command } => {
                    if let Some(computer) = resolve(target) {
                        laser_ops.0.push((
                            computer,
                            match command {
                                LaserCommand::Power(delta) => LaserOp::Power(delta),
                                LaserCommand::Toggle => LaserOp::Toggle,
                            },
                        ));
                    }
                    None
                }
                ActionInput::Apc { target, command } => {
                    if let Some(apc) = resolve(target) {
                        use rs13_classic::apc::ApcCommand as Op;
                        use rs13_classic::power::ChannelMode;
                        let op = match command {
                            ApcCommand::Breaker => Op::Breaker,
                            ApcCommand::ChargeMode => Op::ChargeMode,
                            ApcCommand::CoverLock => Op::CoverLock,
                            ApcCommand::Channel { channel, mode } => Op::Channel(
                                usize::from(channel),
                                match mode {
                                    ApcChannelSetting::Off => ChannelMode::Off,
                                    ApcChannelSetting::On => ChannelMode::On,
                                    ApcChannelSetting::Auto => ChannelMode::AutoOn,
                                },
                            ),
                        };
                        apc_ops.0.push((apc, op));
                    }
                    None
                }
                ActionInput::Respawn => None,
                ActionInput::Resist => {
                    commands
                        .entity(controls.0)
                        .insert(rs13_classic::security::WantsResist(true));
                    None
                }
                ActionInput::Say(text) => {
                    speech.0.push(text);
                    None
                }
                ActionInput::SetName(requested) => {
                    if !name_chosen {
                        match sanitize_name(&requested) {
                            Some(clean) => name.0 = clean,
                            None => outbox
                                .0
                                .push(LocMsg::new("name-rejected").arg("name", name.0.clone())),
                        }
                        commands.entity(controls.0).insert(NameChosen);
                    }
                    None
                }
            };
            // Unknown ids: the entity is gone or never existed. Ignore, as a stale click.
            if let Some(action) = action {
                pending.0.push(action);
            }
        }
    }
}

/// Deliver each mob's queued messages to the client controlling it.
fn send_notices(mut links: Query<(&Controls, &mut MessageSender<Notice>)>, mut outboxes: Query<&mut Outbox>) {
    for (controls, mut sender) in &mut links {
        let Ok(mut outbox) = outboxes.get_mut(controls.0) else {
            continue;
        };
        for msg in outbox.0.drain(..) {
            sender.send::<NoticeChannel>(Notice(msg));
        }
    }
}

/// Send each client the sounds its mob heard. The game has already decided who hears what.
fn send_sounds(mut links: Query<(&Controls, &mut MessageSender<PlaySound>)>, mut ears: Query<&mut Earshot>) {
    for (controls, mut sender) in &mut links {
        let Ok(mut earshot) = ears.get_mut(controls.0) else {
            continue;
        };
        for heard in earshot.0.drain(..) {
            sender.send::<SoundChannel>(PlaySound {
                sound: heard.sound.to_owned(),
                at: heard.at,
                volume: heard.volume.round().clamp(1.0, 100.0) as u8,
            });
        }
    }
}

/// Tell each client its own health, state and intent whenever they change.
fn send_status(
    mut links: Query<(&Controls, &mut SentStatus, &mut MessageSender<StatusUpdate>)>,
    mobs: Query<(
        &Vitals,
        &CombatIntent,
        &Surroundings,
        &Lungs,
        (
            Has<rs13_classic::pulling::Pulling>,
            Option<&rs13_classic::flashes::Dazzled>,
        ),
    )>,
    ghosts: Query<(), With<Ghost>>,
) {
    for (controls, mut sent, mut sender) in &mut links {
        let status = if ghosts.contains(controls.0) {
            // Nothing can hurt a ghost, and no air or warmth matters to it.
            StatusUpdate {
                dazzled: 0,
                health: 100,
                stat: MobStat::Ghost,
                intent: Intent::Help,
                pressure_dkpa: 0,
                alerts: Vec::new(),
                internals: false,
                body_temperature: GHOST_BODY_TEMPERATURE,
                pulling: false,
            }
        } else {
            let Ok((vitals, intent, around, lungs, (pulling, dazzled))) = mobs.get(controls.0) else {
                continue;
            };
            StatusUpdate {
                dazzled: dazzled.map_or(0, |d| d.0),
                // Luna shows health as a whole number.
                health: vitals.health.round() as i32,
                stat: vitals.stat,
                intent: intent.0,
                pressure_dkpa: (around.pressure * 10.0).round().max(0.0) as u32,
                alerts: alerts(around, lungs),
                internals: lungs.internal.is_some(),
                body_temperature: lungs.body_temperature.round().clamp(0.0, 1000.0) as u16,
                pulling,
            }
        };
        if sent.0.as_ref() != Some(&status) {
            sender.send::<NoticeChannel>(status.clone());
            sent.0 = Some(status);
        }
    }
}

/// Send each client the paper windows its mob has opened: a sheet to read, write on or
/// label (`paperwork.md`).
fn send_paper(
    mut links: Query<(&Controls, &mut MessageSender<PaperPanel>)>,
    mut mobs: Query<&mut PaperOut>,
    ids: Query<&NetId>,
) {
    for (controls, mut sender) in &mut links {
        let Ok(mut out) = mobs.get_mut(controls.0) else {
            continue;
        };
        for window in out.0.drain(..) {
            let Ok(id) = ids.get(window.sheet) else { continue };
            sender.send::<NoticeChannel>(PaperPanel(PaperView {
                target: *id,
                title: window.title,
                text: window.text,
                mode: window.mode,
            }));
        }
    }
}

/// Open, update or close each client's machine windows.
fn send_panels(
    mut links: Query<(
        &Controls,
        &mut SentPanel,
        &mut MessageSender<WirePanel>,
        &mut MessageSender<AlarmPanel>,
        &mut MessageSender<StoragePanel>,
        &mut MessageSender<ApcPanel>,
        &mut MessageSender<PowerMonitorPanel>,
        &mut MessageSender<LaserPanel>,
        &mut MessageSender<FreezerPanel>,
        &mut MessageSender<SmesPanel>,
        &mut MessageSender<GasPanel>,
        &mut MessageSender<VendingPanel>,
        &mut MessageSender<CameraPanel>,
        &mut MessageSender<ClipboardPanel>,
        // Nested: a query's data holds at most 15 parts.
        (
            &mut MessageSender<PickListPanel>,
            &mut MessageSender<HeaterPanel>,
            &mut MessageSender<ArcadePanel>,
            &mut MessageSender<BrigPanel>,
            &mut MessageSender<ChemPanel>,
            &mut MessageSender<SleeperPanel>,
            &mut MessageSender<ChemMasterPanel>,
            &mut MessageSender<AutolathePanel>,
            &mut MessageSender<BodyScanPanel>,
            &mut MessageSender<JobPanel>,
            &mut MessageSender<RecordsPanel>,
        ),
    )>,
    panels: Query<(
        &WirePanelOut,
        &AlarmPanelOut,
        &StorageOut,
        &ApcPanelOut,
        &PowerMonitorOut,
        &LaserPanelOut,
        &FreezerPanelOut,
        &SmesPanelOut,
        &GasPanelOut,
        &VendingPanelOut,
        &CameraPanelOut,
        &ClipboardOut,
        &PickListOut,
        (
            &HeaterPanelOut,
            &ArcadePanelOut,
            &BrigPanelOut,
            &ChemPanelOut,
            &SleeperPanelOut,
            &MasterPanelOut,
            &LathePanelOut,
            &ScanPanelOut,
            &JobPanelOut,
            &RecordsPanelOut,
        ),
    )>,
) {
    for (
        controls,
        mut sent,
        mut wires,
        mut alarms,
        mut storages,
        mut apcs,
        mut monitors,
        mut lasers,
        mut freezers,
        mut smes,
        mut gases,
        mut vendings,
        mut camera_panels,
        mut clipboard_panels,
        (
            mut pick_panels,
            mut heater_panels,
            mut arcade_panels,
            mut brig_panels,
            mut chem_panels,
            mut sleeper_panels,
            mut master_panels,
            mut lathe_panels,
            mut scan_panels,
            mut job_panels,
            mut record_panels,
        ),
    ) in &mut links
    {
        let Ok((
            wire_panel,
            alarm_panel,
            storage,
            apc_panel,
            monitor,
            laser,
            freezer,
            smes_panel,
            gas,
            vending,
            camera,
            clipboard,
            pick_list,
            (heater, arcade, brig, chem, sleeper, master, lathe, scan, job, record),
        )) = panels.get(controls.0)
        else {
            continue;
        };
        if sent.2 != storage.0 {
            storages.send::<NoticeChannel>(StoragePanel(storage.0.clone()));
            sent.2 = storage.0.clone();
        }
        if sent.0 != wire_panel.0 {
            wires.send::<NoticeChannel>(WirePanel(wire_panel.0.clone()));
            sent.0 = wire_panel.0.clone();
        }
        if sent.1 != alarm_panel.0 {
            alarms.send::<NoticeChannel>(AlarmPanel(alarm_panel.0.clone()));
            sent.1 = alarm_panel.0.clone();
        }
        if sent.3 != apc_panel.0 {
            apcs.send::<NoticeChannel>(ApcPanel(apc_panel.0.clone()));
            sent.3 = apc_panel.0.clone();
        }
        if sent.4 != monitor.0 {
            monitors.send::<NoticeChannel>(PowerMonitorPanel(monitor.0.clone()));
            sent.4 = monitor.0.clone();
        }
        if sent.5 != laser.0 {
            lasers.send::<NoticeChannel>(LaserPanel(laser.0.clone()));
            sent.5 = laser.0.clone();
        }
        if sent.6 != freezer.0 {
            freezers.send::<NoticeChannel>(FreezerPanel(freezer.0.clone()));
            sent.6 = freezer.0.clone();
        }
        if sent.7 != smes_panel.0 {
            smes.send::<NoticeChannel>(SmesPanel(smes_panel.0.clone()));
            sent.7 = smes_panel.0.clone();
        }
        if sent.8 != gas.0 {
            gases.send::<NoticeChannel>(GasPanel(gas.0.clone()));
            sent.8 = gas.0.clone();
        }
        if sent.9 != vending.0 {
            vendings.send::<NoticeChannel>(VendingPanel(vending.0.clone()));
            sent.9 = vending.0.clone();
        }
        if sent.10 != camera.0 {
            camera_panels.send::<NoticeChannel>(CameraPanel(camera.0.clone()));
            sent.10 = camera.0.clone();
        }
        if sent.11 != clipboard.0 {
            clipboard_panels.send::<NoticeChannel>(ClipboardPanel(clipboard.0.clone()));
            sent.11 = clipboard.0.clone();
        }
        if sent.12 != pick_list.0 {
            pick_panels.send::<NoticeChannel>(PickListPanel(pick_list.0.clone()));
            sent.12 = pick_list.0.clone();
        }
        if sent.13 != heater.0 {
            heater_panels.send::<NoticeChannel>(HeaterPanel(heater.0.clone()));
            sent.13 = heater.0.clone();
        }
        if sent.14 != arcade.0 {
            arcade_panels.send::<NoticeChannel>(ArcadePanel(arcade.0.clone()));
            sent.14 = arcade.0.clone();
        }
        if sent.15 != brig.0 {
            brig_panels.send::<NoticeChannel>(BrigPanel(brig.0.clone()));
            sent.15 = brig.0.clone();
        }
        if sent.16 != chem.0 {
            chem_panels.send::<NoticeChannel>(ChemPanel(chem.0.clone()));
            sent.16 = chem.0.clone();
        }
        if sent.17 != sleeper.0 {
            sleeper_panels.send::<NoticeChannel>(SleeperPanel(sleeper.0.clone()));
            sent.17 = sleeper.0.clone();
        }
        if sent.18 != master.0 {
            master_panels.send::<NoticeChannel>(ChemMasterPanel(master.0.clone()));
            sent.18 = master.0.clone();
        }
        if sent.19 != lathe.0 {
            lathe_panels.send::<NoticeChannel>(AutolathePanel(lathe.0.clone()));
            sent.19 = lathe.0.clone();
        }
        if sent.20 != scan.0 {
            scan_panels.send::<NoticeChannel>(BodyScanPanel(scan.0.clone()));
            sent.20 = scan.0.clone();
        }
        if sent.21 != job.0 {
            job_panels.send::<NoticeChannel>(JobPanel(job.0.clone()));
            sent.21 = job.0.clone();
        }
        if sent.22 != record.0 {
            record_panels.send::<NoticeChannel>(RecordsPanel(record.0.clone()));
            sent.22 = record.0.clone();
        }
    }
}

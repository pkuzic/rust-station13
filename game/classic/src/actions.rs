//! Player and NPC actions other than movement: clicking things, dropping, swapping
//! hands, examining. Spec: `docs/reference/hands.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, DisplayName, GridPos, Hidden, Intent, MobStat, TilePos};

use crate::combat::{CombatIntent, PendingAttacks};
use crate::doors::{Airlock, DoorState};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals};
use crate::machines::{Connector, GasStore, PipeMember, PipeNets, Portable, Switch, Valve};
use crate::movement::Mob;
use crate::power::{Channel, Smes};
use crate::time::WorldTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Click on something with the active hand: pick it up, or use it / the held item on it.
    UseOn(Entity),
    /// Put the active-hand item on the floor.
    Drop,
    SwapHands,
    Examine(Entity),
    /// Look at a wall or floor.
    ExamineTile(TilePos),
    SetIntent(Intent),
    /// Close the open container.
    CloseStorage,
    /// Click on a tile itself, where nothing on it took the click: lay cable with a coil,
    /// pry up floor tiles or put them back.
    UseOnTile(TilePos),
    /// Turn to face a way without stepping.
    Face(Dir),
    /// Start pulling something within a tile (Ctrl-click).
    Pull(Entity),
    /// One of its verbs from the right-click menu (`verbs.rs`).
    Verb(Entity, &'static str),
    StopPulling,
}

/// Messages from a click: one for the mob, one for everyone who can see it (needs the
/// mob's `$name` added).
#[derive(Debug, Default)]
pub struct ClickOutcome {
    pub to_mob: Option<LocMsg>,
    pub seen: Option<LocMsg>,
}

impl ClickOutcome {
    /// Just a message for the mob.
    pub fn told(id: &str) -> Self {
        Self {
            to_mob: Some(LocMsg::new(id)),
            seen: None,
        }
    }
}

/// Actions waiting for the next tick, in the order they were requested.
#[derive(Component, Debug, Default)]
pub struct PendingActions(pub Vec<Action>);

/// Messages for the player controlling this mob, waiting to be sent.
#[derive(Component, Debug, Default)]
pub struct Outbox(pub Vec<LocMsg>);

/// Same tile or one of the eight neighbours, on the same deck.
pub fn in_reach(a: TilePos, b: TilePos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

type ActingMob<'a> = (
    Entity,
    &'a GridPos,
    &'a mut Hands,
    &'a mut PendingActions,
    &'a mut Outbox,
    Option<&'a mut CombatIntent>,
    Option<&'a Vitals>,
    &'a mut Mob,
);

pub fn process_actions(
    time: Res<WorldTime>,
    mut mobs: Query<ActingMob, With<Mob>>,
    mut targets: Query<
        (
            Option<&GridPos>,
            Has<Item>,
            Option<&mut Airlock>,
            Option<&mut Switch>,
            Option<&mut PipeMember>,
            Has<Portable>,
            Has<Valve>,
        ),
        Without<Mob>,
    >,
    ports: Query<&GridPos, (With<Connector>, Without<Mob>)>,
    stores: Query<&GasStore>,
    mut nets: ResMut<PipeNets>,
    batteries: Query<&Smes>,
    people: Query<(), With<Body>>,
    mut attacks: ResMut<PendingAttacks>,
    appearances: Query<(&Appearance, Option<&DisplayName>)>,
    electrified: Query<&crate::wires::Wiring>,
    (mut shocks, mut visible, mut sounds, mut treatments, mut pulls): (
        ResMut<crate::wires::PendingShocks>,
        ResMut<crate::combat::VisibleEvents>,
        ResMut<crate::sound::SoundEvents>,
        ResMut<crate::medical::PendingTreatments>,
        ResMut<crate::pulling::PendingPulls>,
    ),
    (
        mut alarms,
        mut storage,
        mut cable,
        hidden,
        mut welding,
        places,
        mut furniture,
        meters,
        mut lights,
        mut closets,
        inside,
        mut firedoors,
        mut flashlights,
        mut firealarms,
        (
            mut disposals,
            mut extinguishers,
            mut walllockers,
            gas_windows,
            vendors,
            mut food,
            mut smoking,
            inventory,
            (
                consoles,
                mut cameras,
                mut paperwork,
                mut windoors,
                mut filing,
                mut linen,
                (
                    mut belt_switches,
                    belts,
                    pulling,
                    mut heaters,
                    morgues,
                    trays,
                    crema_switches,
                    shield_parts,
                    knocked_down,
                    flashers,
                    microwaves,
                    arcades,
                    airlock_parts,
                    (
                        scanners,
                        cloning_consoles,
                        door_timers,
                        rechargers,
                        cuffed,
                        turret_controls,
                        reagent_holders,
                        dispensers_chem,
                        (
                            medbay_sleepers,
                            medbay_consoles,
                            medbay_cells,
                            chem_masters,
                            bullet_guns,
                            fill_tanks,
                            lathes,
                            body_scanners,
                            scan_consoles,
                            records_consoles,
                            id_computers,
                            plant_pots,
                            barriers,
                            comms_consoles,
                        ),
                    ),
                ),
            ),
        ),
        (
            mut generators,
            mut dispensers,
            mut chargers,
            mut jetpacks,
            mut radios,
            mut chairs,
            mut apc_work,
            monitors,
            tegs,
            laser_computers,
            freezers,
            smes_units,
            mut igniters,
            mut floodlights,
            mut blast_doors,
            climbing,
        ),
    ): (
        Query<(&mut crate::alarms::AirAlarm, &crate::power::PowerUser)>,
        crate::storage::StorageAccess,
        crate::cables::CableWork,
        Query<(), With<Hidden>>,
        crate::welding::WeldWork,
        Query<&GridPos>,
        crate::furniture::FurnitureWork,
        Query<(&GridPos, &crate::meters::Meter, &crate::power::PowerUser)>,
        crate::lights::LightWork,
        crate::closets::ClosetWork,
        Query<(), With<crate::closets::InCloset>>,
        crate::firedoors::FiredoorWork,
        crate::flashlights::FlashlightWork,
        crate::firealarms::FireAlarmWork,
        (
            crate::disposals::DisposalWork,
            crate::extinguishers::ExtinguisherWork,
            crate::walllockers::WallLockerWork,
            crate::gas_panel::GasPanelWork,
            Query<(&GridPos, &crate::power::PowerUser), With<crate::vending::Vending>>,
            crate::food::FoodWork,
            crate::smoking::SmokingWork,
            Query<&crate::equipment::WornBy>,
            (
                Query<(&GridPos, &crate::power::PowerUser), With<crate::cameras::SecurityConsole>>,
                Query<(&GridPos, &mut crate::cameras::Camera)>,
                crate::paperwork::PaperWork,
                Query<&mut crate::windoors::Windoor>,
                crate::filing::FilingWork,
                crate::bedsheets::LinenWork,
                (
                    Query<&mut crate::conveyors::ConveyorSwitch>,
                    Query<(), With<crate::conveyors::Conveyor>>,
                    Query<&crate::pulling::Pulling>,
                    crate::heaters::HeaterWork,
                    Query<(), With<crate::morgue::Morgue>>,
                    Query<&crate::morgue::MorgueTray>,
                    Query<(), With<crate::morgue::Igniter>>,
                    Query<
                        (),
                        Or<(
                            With<crate::shields::ShieldsButton>,
                            With<crate::shields::ShieldConverter>,
                            With<crate::shields::ShieldCapacitor>,
                        )>,
                    >,
                    Query<(), With<crate::flashes::Weakened>>,
                    Query<(), With<crate::flashes::Flasher>>,
                    Query<(), With<crate::microwaves::Microwave>>,
                    Query<(), With<crate::arcade::Arcade>>,
                    Query<
                        (),
                        Or<(
                            With<crate::airlock_control::AccessButton>,
                            With<crate::airlock_control::AirlockConsole>,
                        )>,
                    >,
                    (
                        Query<(), With<crate::cloning::DnaScanner>>,
                        Query<(), With<crate::cloning::CloningConsole>>,
                        Query<(), With<crate::brig::DoorTimer>>,
                        Query<(), With<crate::weapons::Recharger>>,
                        Query<(), With<crate::security::Cuffed>>,
                        Query<(), With<crate::security::TurretControl>>,
                        Query<(), With<crate::chemistry::Reagents>>,
                        Query<(), With<crate::chem_dispenser::ChemDispenser>>,
                        (
                            Query<(), With<crate::medbay::Sleeper>>,
                            Query<(), With<crate::medbay::SleeperConsole>>,
                            Query<(), With<crate::medbay::CryoCell>>,
                            Query<(), With<crate::chem_master::ChemMaster>>,
                            Query<(), With<crate::weapons::Caliber>>,
                            Query<
                                (),
                                Or<(
                                    With<crate::reagent_tanks::ReagentTank>,
                                    With<crate::extinguishers::WaterTank>,
                                    With<crate::welding::FuelTank>,
                                )>,
                            >,
                            Query<(), With<crate::autolathe::Autolathe>>,
                            Query<(), With<crate::medbay::BodyScanner>>,
                            Query<(), With<crate::medbay::ScannerConsole>>,
                            Query<(), With<crate::records::RecordsConsole>>,
                            Query<(), With<crate::id_computer::IdComputer>>,
                            Query<(), With<crate::hydroponics::PlantPot>>,
                            Query<(), With<crate::security::Barrier>>,
                            Query<(), With<crate::escape::CommsConsole>>,
                        ),
                    ),
                ),
            ),
        ),
        (
            crate::generators::GeneratorWork,
            crate::dispensers::DispenserWork,
            crate::cells::ChargerWork,
            crate::jetpacks::JetpackWork,
            crate::radio::RadioWork,
            crate::chairs::ChairWork,
            crate::apc::ApcWork,
            Query<(&GridPos, &crate::power::PowerUser), With<crate::power_monitor::PowerMonitor>>,
            Query<&crate::teg::Teg>,
            Query<(&GridPos, &crate::power::PowerUser), With<crate::supermatter::LaserComputer>>,
            Query<&GridPos, With<crate::freezers::Freezer>>,
            Query<&GridPos, With<crate::power::Smes>>,
            crate::igniters::IgniterWork,
            crate::floodlights::FloodlightWork,
            crate::blastdoors::BlastDoorWork,
            crate::multiz::ClimbWork,
        ),
    ),
    mut commands: Commands,
) {
    let now = time.deciseconds();
    // Items picked up earlier this tick (component changes apply after the system).
    let mut taken: HashSet<Entity> = HashSet::new();
    // Lit welding tools used on something in reach this tick, by whom and where.
    let mut welded_with: Vec<(Entity, Entity, TilePos)> = Vec::new();

    for (mob, mob_pos, mut hands, mut pending, mut outbox, mut intent, vitals, mut mob_state) in &mut mobs {
        // Cuffed hands can't do anything (`security.md`).
        if knocked_down.contains(mob)
            || cuffed.contains(mob)
            || vitals.is_some_and(|v| v.stat != MobStat::Conscious)
        {
            pending.0.clear();
            continue;
        }
        for action in pending.0.drain(..) {
            // Things under floor tiles can't be seen or touched.
            if let Action::UseOn(target) | Action::Examine(target) = action
                && hidden.contains(target)
            {
                continue;
            }
            // From inside a closet nothing outside can be reached (`closets.md`).
            if inside.contains(mob)
                && matches!(action, Action::UseOn(_) | Action::UseOnTile(_) | Action::Drop)
            {
                continue;
            }
            // The tool in the active hand, for its sound (`sound.md`).
            let tool: Option<String> = hands
                .active_item()
                .and_then(|e| appearances.get(e).ok())
                .map(|(a, _)| a.sprite.clone());
            let tool = tool.as_deref();
            // A lit welding tool burns fuel on whatever it is used on (`welding.md`).
            let reaches = match action {
                Action::UseOn(target) => {
                    hands.active_item() != Some(target)
                        && places.get(target).map_or_else(
                            |_| hands.left == Some(target) || hands.right == Some(target),
                            |p| in_reach(mob_pos.0, p.0),
                        )
                }
                Action::UseOnTile(tile) => in_reach(mob_pos.0, tile),
                _ => false,
            };
            if reaches
                && let Some(active) = hands.active_item()
                && welding.lit(Some(active)).is_some()
            {
                let at = match action {
                    Action::UseOnTile(tile) => tile,
                    Action::UseOn(target) => places.get(target).map_or(mob_pos.0, |p| p.0),
                    _ => mob_pos.0,
                };
                welded_with.push((mob, active, at));
            }
            match action {
                Action::UseOn(target)
                    if comms_consoles.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The communications console (`round.md`).
                    commands.queue(move |world: &mut World| crate::escape::click(world, mob, target));
                }
                Action::UseOn(target)
                    if barriers.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Deployable barriers (`security.md`).
                    commands
                        .queue(move |world: &mut World| crate::security::click_barrier(world, mob, target));
                }
                Action::UseOn(target)
                    if plant_pots.contains(target)
                        && !tool.is_some_and(crate::chemistry::is_chem_tool)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Plant pots (`hydroponics.md`). Glassware pours water into them instead.
                    commands.queue(move |world: &mut World| crate::hydroponics::click(world, mob, target));
                }
                Action::UseOn(target)
                    if id_computers.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The identification computer (`access.md`).
                    commands.queue(move |world: &mut World| crate::id_computer::click(world, mob, target));
                }
                Action::UseOn(target)
                    if records_consoles.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Crew records (`records.md`).
                    commands.queue(move |world: &mut World| crate::records::click(world, mob, target));
                }
                Action::UseOn(target)
                    if (body_scanners.contains(target) || scan_consoles.contains(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The body scanner and its console (`medbay.md`).
                    let scanner = body_scanners.contains(target);
                    commands.queue(move |world: &mut World| {
                        if scanner {
                            crate::medbay::click_body_scanner(world, mob, target);
                        } else {
                            crate::medbay::click_scan_console(world, mob, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if lathes.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The autolathe (`autolathe.md`).
                    commands.queue(move |world: &mut World| crate::autolathe::click(world, mob, target));
                }
                Action::UseOn(target)
                    if bullet_guns.contains(target)
                        && tool.is_some_and(crate::weapons::is_ammo)
                        && (hands.left == Some(target)
                            || hands.right == Some(target)
                            || places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0))) =>
                {
                    // Loading a bullet gun (`weapons.md`).
                    commands.queue(move |world: &mut World| crate::weapons::load(world, mob, target));
                }
                Action::UseOn(target)
                    if fill_tanks.contains(target)
                        && tool.is_some_and(crate::chemistry::is_glass)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Tanks to fill a beaker from (`chemistry.md`).
                    commands.queue(move |world: &mut World| crate::reagent_tanks::fill(world, mob, target));
                }
                Action::UseOn(target)
                    if chem_masters.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The CheMaster (`chemistry.md`).
                    commands.queue(move |world: &mut World| crate::chem_master::click(world, mob, target));
                }
                Action::UseOn(target)
                    if (medbay_sleepers.contains(target)
                        || medbay_consoles.contains(target)
                        || medbay_cells.contains(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Sleepers and cryo cells (`medbay.md`).
                    let (sleeper, console) =
                        (medbay_sleepers.contains(target), medbay_consoles.contains(target));
                    commands.queue(move |world: &mut World| {
                        if sleeper {
                            crate::medbay::click_sleeper(world, mob, target);
                        } else if console {
                            crate::medbay::click_console(world, mob, target);
                        } else {
                            crate::medbay::click_cryo(world, mob, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if dispensers_chem.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    commands.queue(move |world: &mut World| crate::chem_dispenser::click(world, mob, target));
                }
                // Syringes, pills, hyposprays and glassware (`chemistry.md`).
                Action::UseOn(target)
                    if tool.is_some_and(crate::chemistry::is_chem_tool)
                        && (reagent_holders.contains(target) || people.contains(target))
                        && (hands.active_item() == Some(target)
                            || places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0))
                            || target == mob) =>
                {
                    let proto = tool.unwrap_or_default().to_owned();
                    let held = hands.active_item();
                    commands.queue(move |world: &mut World| {
                        let Some(held) = held else { return };
                        if crate::chemistry::is_syringe(&proto) {
                            crate::chemistry::use_syringe(world, mob, held, target);
                        } else if target == held {
                            // Pills are for people; the rest have nothing to do by themselves.
                        } else if crate::chemistry::is_pill(&proto) {
                            crate::chemistry::use_pill(world, mob, held, target);
                        } else if crate::chemistry::is_hypo(&proto) {
                            crate::chemistry::use_hypo(world, mob, held, target);
                        } else {
                            crate::chemistry::pour(world, mob, held, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if tool == Some(crate::security::HANDCUFFS)
                        && people.contains(target)
                        && target != mob
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    commands
                        .queue(move |world: &mut World| crate::security::start_cuffing(world, mob, target));
                }
                Action::UseOn(target)
                    if tool.is_none()
                        && cuffed.contains(target)
                        && target != mob
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    commands
                        .queue(move |world: &mut World| crate::security::start_uncuffing(world, mob, target));
                }
                Action::UseOn(target)
                    if turret_controls.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    commands
                        .queue(move |world: &mut World| crate::security::click_control(world, mob, target));
                }
                // Guns and batons first: they answer clicks of their own (`weapons.md`).
                Action::UseOn(target)
                    if tool.is_some_and(crate::weapons::is_gun) && hands.active_item() == Some(target) =>
                {
                    commands.queue(move |world: &mut World| crate::weapons::switch_gun(world, mob, target));
                }
                Action::UseOn(target)
                    if tool.is_some_and(crate::weapons::is_gun)
                        && places.get(target).is_ok_and(|p| !in_reach(mob_pos.0, p.0)) =>
                {
                    let gun = hands.active_item();
                    let to = places.get(target).map_or(mob_pos.0, |p| p.0);
                    commands.queue(move |world: &mut World| {
                        if let Some(gun) = gun {
                            crate::weapons::fire(world, mob, gun, to);
                        }
                    });
                }
                Action::UseOnTile(to)
                    if tool.is_some_and(crate::weapons::is_gun) && !in_reach(mob_pos.0, to) =>
                {
                    let gun = hands.active_item();
                    commands.queue(move |world: &mut World| {
                        if let Some(gun) = gun {
                            crate::weapons::fire(world, mob, gun, to);
                        }
                    });
                }
                Action::UseOn(target)
                    if tool == Some(crate::weapons::BATON) && hands.active_item() == Some(target) =>
                {
                    commands.queue(move |world: &mut World| crate::weapons::toggle_baton(world, mob, target));
                }
                Action::UseOn(target)
                    if tool == Some(crate::weapons::BATON)
                        && people.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    let baton = hands.active_item();
                    // On harm intent it's also a blow like any other.
                    if intent.as_deref().is_some_and(|i| i.0 == rs13_world::Intent::Harm) {
                        attacks.0.push((mob, target));
                    }
                    commands.queue(move |world: &mut World| {
                        if let Some(baton) = baton {
                            crate::weapons::baton_hit(world, mob, baton, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if tool == Some(crate::flashes::FLASH)
                        && (hands.active_item() == Some(target)
                            || (people.contains(target)
                                && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)))) =>
                {
                    // Flashes (`flashes.md`).
                    let flash = hands.active_item();
                    commands.queue(move |world: &mut World| {
                        let Some(flash) = flash else { return };
                        if flash == target {
                            crate::flashes::flash_around(world, mob, flash);
                        } else {
                            crate::flashes::flash_someone(world, mob, flash, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if flashers.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0))
                        && matches!(tool, Some("wirecutters" | "wrench")) =>
                {
                    let wrench = tool == Some("wrench");
                    commands.queue(move |world: &mut World| {
                        if wrench {
                            crate::flashes::wrench_portable(world, mob, target);
                        } else {
                            crate::flashes::cut_bulb(world, mob, target);
                        }
                    });
                }
                Action::SwapHands => hands.active = hands.active.other(),
                // Turn on the spot; a chair turns its sitter back (`movement::sync_facing`).
                Action::Face(dir) => mob_state.facing = Some(dir),
                Action::Pull(target) => pulls.0.push((mob, target)),
                Action::Verb(target, verb) => {
                    // Luna's object verbs (`verbs.rs`): things within a tile, or carried.
                    let near = places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0));
                    let carried = hands.left == Some(target)
                        || hands.right == Some(target)
                        || inventory.get(target).is_ok_and(|w| w.0 == mob);
                    let outcome = match verb {
                        crate::verbs::TOGGLE_OPEN if near => {
                            // As an empty hand would (Luna calls `attack_hand`).
                            let mut bare = *hands;
                            *bare.slot_mut(bare.active) = None;
                            closets.use_on(mob, &mut bare, false, target, &mut commands)
                        }
                        crate::verbs::TOGGLE_LOCK
                            if near && places.get(target).is_ok_and(|p| p.0 != mob_pos.0) =>
                        {
                            closets.toggle_lock(mob, &hands, target)
                        }
                        crate::verbs::ROTATE_CHAIR if near => chairs.rotate(target),
                        crate::verbs::TOGGLE_LIGHT if carried => flashlights.use_self(target),
                        crate::verbs::TOGGLE_JETPACK if carried => jetpacks.use_self(target),
                        _ => None,
                    };
                    if let Some(outcome) = outcome {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                    }
                }
                Action::ExamineTile(at) => {
                    commands.queue(move |world: &mut World| {
                        let mut maps = world.query::<&rs13_world::MapLayer>();
                        let id = maps.iter(world).find_map(|m| m.0.tile(at)).map(|t| t.id.clone());
                        if let Some(id) = id
                            && let Some(mut outbox) = world.get_mut::<Outbox>(mob)
                        {
                            outbox.0.push(LocMsg::new("examine").arg("item", id));
                        }
                    });
                }
                Action::StopPulling => {
                    commands.entity(mob).remove::<crate::pulling::Pulling>();
                }
                Action::CloseStorage => {
                    commands.entity(mob).remove::<crate::storage::OpenStorage>();
                }
                Action::SetIntent(new) => {
                    if let Some(intent) = intent.as_mut() {
                        intent.0 = new;
                    }
                }
                Action::Drop => {
                    let active = hands.active;
                    if let Some(item) = hands.slot_mut(active).take() {
                        commands
                            .entity(item)
                            .remove::<HeldBy>()
                            .insert(GridPos(mob_pos.0));
                    }
                }
                Action::Examine(target) => match appearances.get(target) {
                    Ok((_, Some(name))) => {
                        outbox
                            .0
                            .push(LocMsg::new("examine-person").arg("name", name.0.clone()));
                    }
                    Ok((appearance, None)) => {
                        outbox
                            .0
                            .push(LocMsg::new("examine").arg("item", appearance.sprite.clone()));
                        if let Some(apc) = apc_work.get(target) {
                            let mode = |c: Channel| {
                                if apc.operating && apc.channels[c as usize].is_on() {
                                    "on"
                                } else {
                                    "off"
                                }
                            };
                            outbox.0.push(
                                LocMsg::new("examine-apc")
                                    .num("value", apc.cell_percent().round() as i64)
                                    .arg("equipment", mode(Channel::Equipment))
                                    .arg("lighting", mode(Channel::Lighting))
                                    .arg("environment", mode(Channel::Environment)),
                            );
                        }
                        if reagent_holders.contains(target) {
                            commands.queue(move |world: &mut World| {
                                if let Some(more) = crate::chemistry::examine(world, target)
                                    && let Some(mut outbox) = world.get_mut::<Outbox>(mob)
                                {
                                    outbox.0.push(more);
                                }
                            });
                        }
                        if shield_parts.contains(target) {
                            commands.queue(move |world: &mut World| {
                                if let Some(more) = crate::hydroponics::examine(world, target)
                                    && let Some(mut outbox) = world.get_mut::<Outbox>(mob)
                                {
                                    outbox.0.push(more);
                                }
                                if let Some(more) = crate::id_computer::examine(world, target)
                                    && let Some(mut outbox) = world.get_mut::<Outbox>(mob)
                                {
                                    outbox.0.push(more);
                                }
                                if let Some(more) = crate::shields::examine(world, target)
                                    && let Some(mut outbox) = world.get_mut::<Outbox>(mob)
                                {
                                    outbox.0.push(more);
                                }
                            });
                        }
                        if let Some(more) = cable.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = cable.floors.examine(target) {
                            outbox.0.push(more);
                        }
                        paperwork.examine(mob, target);
                        if let Some(more) = paperwork.examine_bin(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = heaters.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = linen.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = food.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = smoking.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = extinguishers.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = welding.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = crate::meters::examine(target, &meters) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = lights.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = generators.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = dispensers.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = chargers.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Ok(teg) = tegs.get(target) {
                            outbox.0.push(crate::teg::examine(teg));
                        }
                        if let Ok(smes) = batteries.get(target) {
                            let percent = (100.0 * smes.charge / smes.capacity).round() as i64;
                            outbox.0.push(LocMsg::new("examine-smes").num("value", percent));
                        }
                        // Anything holding gas has a pressure gauge.
                        if let Ok((_, _, _, _, Some(member), ..)) = targets.get(target) {
                            let kpa = nets.gauge(target, stores.get(target).ok(), member.volume);
                            outbox
                                .0
                                .push(LocMsg::new("examine-gauge").num("value", kpa.round() as i64));
                        }
                    }
                    Err(_) => {}
                },
                Action::UseOnTile(tile) if extinguishers.takes_click(hands.active_item(), None, false) => {
                    // A fire extinguisher sprays at any range on its deck (`extinguishers.md`).
                    if tile.z != mob_pos.0.z {
                        continue;
                    }
                    if let Some(item) = hands.active_item()
                        && let Some(outcome) = extinguishers.use_on(item, None, false, mob_pos.0, tile, now)
                    {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target)
                    if hands.active_item() != Some(target)
                        && extinguishers.takes_click(
                            hands.active_item(),
                            Some(target),
                            places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)),
                        )
                        && !(extinguishers.is_cabinet(target)
                            && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0))) =>
                {
                    // A fire extinguisher refills at a water tank in reach, and with its
                    // safety off sprays at whatever is clicked, at any range on its deck
                    // (`extinguishers.md`).
                    let Ok(at) = places.get(target).map(|p| p.0) else {
                        continue;
                    };
                    if at.z != mob_pos.0.z {
                        continue;
                    }
                    let near = in_reach(mob_pos.0, at);
                    if let Some(item) = hands.active_item()
                        && let Some(outcome) =
                            extinguishers.use_on(item, Some(target), near, mob_pos.0, at, now)
                    {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if extinguishers.is_cabinet(target) => {
                    // Extinguisher cabinets (`extinguishers.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = extinguishers.use_cabinet(mob, target, &mut hands, &mut commands) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOnTile(tile) => {
                    // Cable coils (`cables.md`), then crowbars and floor tiles (`floors.md`).
                    let outcome = cable
                        .use_on_tile((mob_pos.0, mob_state.facing), &mut hands, tile, &mut commands)
                        .or_else(|| {
                            cable
                                .floors
                                .use_on_tile(mob_pos.0, &mut hands, tile, &mut commands)
                        });
                    if let Some(outcome) = outcome {
                        sounds.tool_used(tool, tile);
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target)
                    if smoking.takes(
                        mob,
                        &hands,
                        target,
                        appearances.get(target).is_ok_and(|(a, _)| a.sprite == "matchbox"),
                        welding.lit(hands.active_item()).is_some(),
                    ) && places.get(target).map_or(true, |p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Matches struck, cigarettes lit, a cigarette taken out of its packet
                    // (`smoking.md`).
                    let welder_lit = welding.lit(hands.active_item()).is_some();
                    if let Some(outcome) = smoking.use_on(mob, &mut hands, target, welder_lit, &mut commands)
                    {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                    }
                }
                Action::UseOn(target)
                    if paperwork.takes(&hands, target)
                        && places.get(target).map_or(true, |p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Paper bins, writing with a pen, clipboards (`paperwork.md`).
                    if let Some(outcome) = paperwork.use_on(mob, &mut hands, target, &mut commands) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target)
                    if (filing.contains(target) || linen.is_bin(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Filing cabinets (`filing-cabinets.md`), linen bins (`bedsheets.md`).
                    let at = places.get(target).map_or(mob_pos.0, |p| p.0);
                    let wrench = holding(&hands, &appearances, "wrench");
                    if let Some((outcome, wrenched)) =
                        filing.use_on(mob, &mut hands, target, wrench, &mut commands)
                    {
                        if wrenched {
                            sounds.play(crate::sound::RATCHET, at, 50.0);
                        }
                        outbox.0.extend(outcome.to_mob);
                    } else if let Some(outcome) =
                        linen.use_on(mob, mob_pos.0, &mut hands, target, &mut commands)
                    {
                        outbox.0.extend(outcome.to_mob);
                        outbox.0.extend(outcome.seen);
                    }
                }
                Action::UseOn(target)
                    if (morgues.contains(target) || trays.contains(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Morgue drawers and their trays (`morgue.md`).
                    let at = places.get(target).map_or(mob_pos.0, |p| p.0);
                    let pulled = pulling.get(mob).ok().map(|p| p.0).filter(|p| people.contains(*p));
                    if trays.contains(target)
                        && let Some(body) = pulled
                    {
                        // Luna `m_tray/MouseDrop_T`: whoever you pull is laid on the tray.
                        if places.get(body).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                            commands.entity(body).insert(GridPos(at));
                            commands.entity(mob).remove::<crate::pulling::Pulling>();
                            let name = |e: Entity| {
                                appearances
                                    .get(e)
                                    .ok()
                                    .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                    .unwrap_or_default()
                            };
                            let tray_noun = appearances
                                .get(target)
                                .map_or(crate::morgue::MORGUE_TRAY.to_owned(), |(a, _)| a.sprite.clone());
                            visible.0.push((
                                mob_pos.0,
                                LocMsg::new("morgue-stuffed")
                                    .arg("name", name(mob))
                                    .arg("victim", name(body))
                                    .arg("item", tray_noun),
                                None,
                            ));
                        }
                    } else if hands.active_item().is_none() {
                        let morgue = trays.get(target).map_or(target, |t| t.0);
                        commands.queue(move |world: &mut World| {
                            if crate::morgue::is_locked(world, morgue) {
                                // Luna `crematorium/attack_hand` while it burns.
                                if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
                                    outbox.0.push(LocMsg::new("crematorium-locked"));
                                }
                                return;
                            }
                            let out = world
                                .get::<crate::morgue::Morgue>(morgue)
                                .is_some_and(|d| d.tray.is_some());
                            if out {
                                crate::morgue::close_tray(world, morgue);
                            } else {
                                crate::morgue::open_tray(world, morgue);
                            }
                        });
                    }
                }
                Action::UseOn(target)
                    if rechargers.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    commands
                        .queue(move |world: &mut World| crate::weapons::click_recharger(world, mob, target));
                }
                Action::UseOn(target)
                    if door_timers.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Brig door timers (`brig.md`).
                    commands.queue(move |world: &mut World| crate::brig::click(world, mob, target));
                }
                Action::UseOn(target)
                    if (scanners.contains(target) || cloning_consoles.contains(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Cloning (`cloning.md`).
                    let scanner = scanners.contains(target);
                    commands.queue(move |world: &mut World| {
                        if scanner {
                            crate::cloning::click_scanner(world, mob, target);
                        } else {
                            crate::cloning::click_console(world, mob, target);
                        }
                    });
                }
                Action::UseOn(target)
                    if airlock_parts.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Cycling airlocks (`airlock-control.md`).
                    commands
                        .queue(move |world: &mut World| crate::airlock_control::click(world, mob, target));
                }
                Action::UseOn(target)
                    if arcades.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Arcade machines (`arcade.md`).
                    commands.entity(mob).insert(crate::wires::PanelViewer(target));
                }
                Action::UseOn(target)
                    if microwaves.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Microwaves (`microwaves.md`).
                    commands.queue(move |world: &mut World| crate::microwaves::click(world, mob, target));
                }
                Action::UseOn(target)
                    if shield_parts.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // The shield generator's button and converters (`shields.md`).
                    commands.queue(move |world: &mut World| crate::shields::click(world, mob, target));
                }
                Action::UseOn(target)
                    if crema_switches.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Crematorium igniters (`morgue.md`): anything in the hand works as a hand.
                    let access = [crate::morgue::IGNITER_ACCESS.to_owned()];
                    if closets.shown(mob, &hands).opens(&access) {
                        commands.queue(move |world: &mut World| crate::morgue::ignite(world, target));
                    } else {
                        outbox.0.push(LocMsg::new("crematorium-denied"));
                    }
                }
                Action::UseOn(target)
                    if heaters.contains(target)
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Space heaters (`space-heaters.md`).
                    let at = places.get(target).map_or(mob_pos.0, |p| p.0);
                    let screwdriver = holding(&hands, &appearances, "screwdriver");
                    if let Some(outcome) = heaters
                        .use_on(mob, &mut hands, target, screwdriver, &mut commands)
                        .flatten()
                    {
                        if screwdriver {
                            sounds.tool_used(tool, at);
                        }
                        outbox.0.push(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), Some(mob)));
                        }
                    }
                }
                Action::UseOn(target)
                    if (belts.contains(target) || belt_switches.contains(target))
                        && places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) =>
                {
                    // Conveyor belts and switches (`conveyors.md`).
                    let at = places.get(target).map_or(mob_pos.0, |p| p.0);
                    let active_hand = hands.active;
                    if belt_switches.contains(target) {
                        if hands.active_item().is_none() {
                            crate::conveyors::throw_switch(target, &mut belt_switches);
                        }
                    } else if let Some(item) = hands.slot_mut(active_hand).take() {
                        // Luna `conveyor/attackby`: the item is put down on the belt.
                        commands.entity(item).remove::<HeldBy>().insert(GridPos(at));
                    } else if let Ok(pulled) = pulling.get(mob)
                        && let Ok(from) = places.get(pulled.0)
                    {
                        // Luna `conveyor/attack_hand`: what you pull is put on the belt, and
                        // you let go.
                        let step = TilePos::new(
                            from.0.x + (at.x - from.0.x).signum(),
                            from.0.y + (at.y - from.0.y).signum(),
                            from.0.z,
                        );
                        commands.entity(pulled.0).insert(GridPos(step));
                        commands.entity(mob).remove::<crate::pulling::Pulling>();
                    }
                }
                Action::UseOn(target) if people.contains(target) && food.is_food(hands.active_item()) => {
                    // Eating and drinking, or feeding someone (`food.md`).
                    let target_pos = places.get(target).map_or(mob_pos.0, |p| p.0);
                    let target_name = appearances
                        .get(target)
                        .ok()
                        .and_then(|(_, n)| n.map(|n| n.0.clone()))
                        .unwrap_or_default();
                    if let Some(outcome) = food.use_on(
                        mob,
                        mob_pos.0,
                        target,
                        target_pos,
                        &target_name,
                        &mut hands,
                        (&mut lights.rng, &mut sounds),
                        &mut commands,
                    ) {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                    }
                }
                Action::UseOn(target)
                    if people.contains(target) && tool.is_some_and(crate::medical::treats) =>
                {
                    // Medical packs and analyzers treat or scan instead of hitting
                    // (`medical.md`); reach is checked when they are resolved.
                    treatments.0.push((mob, target));
                }
                Action::UseOn(target) if people.contains(target) => {
                    // Reach and the outcome are worked out by `resolve_attacks`.
                    attacks.0.push((mob, target));
                }
                Action::UseOn(target)
                    if hands.active_item() == Some(target)
                        && (welding.is_welder(target)
                            || furniture.uses_self(target)
                            || flashlights.contains(target)
                            || jetpacks.contains(target)
                            || radios.contains(target)
                            || extinguishers.is_extinguisher(target)
                            || smoking.uses_self(target)
                            || paperwork.uses_self(target)
                            || linen.is_sheet(target)) =>
                {
                    // A bedsheet spread or pulled off where you stand (`bedsheets.md`).
                    if linen.use_self(mob_pos.0, &mut hands, target, &mut commands) {
                        continue;
                    }
                    // A sheet's label box, a clipboard's window (`paperwork.md`).
                    if paperwork.uses_self(target) {
                        if let Some(outcome) = paperwork.use_self(mob, target, &mut commands) {
                            outbox.0.extend(outcome.to_mob);
                        }
                        continue;
                    }
                    // A lighter lit or shut, a lit cigarette trodden out (`smoking.md`).
                    if smoking.uses_self(target) {
                        if let Some(outcome) = smoking.use_self(
                            mob,
                            mob_pos.0,
                            &mut hands,
                            target,
                            &mut lights.rng,
                            &mut commands,
                        ) {
                            outbox.0.extend(outcome.to_mob);
                            if let Some(seen) = outcome.seen {
                                let name = appearances
                                    .get(mob)
                                    .ok()
                                    .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                    .unwrap_or_default();
                                visible.0.push((mob_pos.0, seen.arg("name", name), None));
                            }
                        }
                        continue;
                    }
                    // The item in the active hand used on itself (Luna `attack_self`): a
                    // welding tool lights or goes out, rack parts become a rack, a
                    // flashlight switches.
                    if let Some(outcome) = welding
                        .use_self(target)
                        .or_else(|| flashlights.use_self(target))
                        .or_else(|| jetpacks.use_self(target))
                        .or_else(|| radios.toggle_microphone(target))
                        .or_else(|| extinguishers.use_self(target))
                    {
                        outbox.0.extend(outcome.to_mob);
                    }
                    furniture.use_self(mob_pos.0, &mut hands, target, &mut commands);
                }
                Action::UseOn(target) if meters.contains(target) => {
                    // A meter reads out to anyone clicking it nearby, whatever they hold
                    // (`meters.md`).
                    if let Some(said) = crate::meters::click(mob_pos.0, target, &meters) {
                        outbox.0.extend(said);
                    }
                }
                Action::UseOn(target) if firedoors.contains(target) => {
                    // Firedoors (`firedoors.md`), within reach: a crowbar or a lit welding tool.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    let crowbar = holding(&hands, &appearances, "crowbar");
                    let lit = welding.lit(hands.active_item()).is_some();
                    if let Some(outcome) = firedoors.use_on(target, crowbar, lit) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if firealarms.contains(target) => {
                    // Fire alarms (`fire-alarm.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    let held = tool.map(str::to_owned);
                    let name = appearances
                        .get(mob)
                        .ok()
                        .and_then(|(_, n)| n.map(|n| n.0.clone()))
                        .unwrap_or_default();
                    if let Some(outcome) = firealarms.use_on(target, held.as_deref(), &name) {
                        outbox.0.extend(outcome.to_mob.to_mob);
                        if let Some(seen) = outcome.seen {
                            sounds.tool_used(tool, mob_pos.0);
                            visible.0.push((mob_pos.0, seen, None));
                        }
                    }
                }
                Action::UseOn(target) if radios.is_intercom(target) && tool.is_none() => {
                    // An empty hand on an intercom works its microphone (`radio.md`), within
                    // reach. A radio on the floor is picked up like anything else.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = radios.toggle_microphone(target) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if smes_units.contains(target) && tool.is_none() => {
                    // An empty hand on a SMES opens its window (`power.md`).
                    crate::smes_panel::open(&smes_units, mob, mob_pos.0, target, &mut commands);
                }
                Action::UseOn(target) if freezers.contains(target) => {
                    // Freezers (`engine.md`).
                    crate::freezers::open(&freezers, mob, mob_pos.0, target, &mut commands);
                }
                Action::UseOn(target) if laser_computers.contains(target) => {
                    // The laser control computer (`engine.md`).
                    crate::supermatter::open_computer(
                        &laser_computers,
                        mob,
                        mob_pos.0,
                        target,
                        &mut commands,
                    );
                }
                Action::UseOn(target) if monitors.contains(target) => {
                    // Power monitoring computers (`power.md`).
                    crate::power_monitor::open(&monitors, mob, mob_pos.0, target, &mut commands);
                }
                Action::UseOn(target) if apc_work.contains(target) => {
                    // APCs (`power.md`), within reach.
                    let Ok(at) = places.get(target).map(|p| p.0) else {
                        continue;
                    };
                    if !in_reach(mob_pos.0, at) {
                        continue;
                    }
                    if let Some(outcome) = apc_work.use_on(mob, target, tool, &mut hands, at, &mut commands) {
                        sounds.tool_used(tool, at);
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if disposals.contains(target) => {
                    // Disposal bins (`disposals.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    let held = hands
                        .active_item()
                        .and_then(|i| appearances.get(i).ok())
                        .map(|(a, _)| a.sprite.clone());
                    if let Some((outcome, seen)) =
                        disposals.use_on(target, &mut hands, held.as_deref(), &mut commands)
                    {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), Some(mob)));
                        }
                    }
                }
                Action::UseOn(target) if climbing.is_ladder(target) => {
                    // Ladders (`multiz.md`), within reach: climb to the other end.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(to) = climbing.climb(target) {
                        commands.entity(mob).insert(GridPos(to));
                    }
                }
                Action::UseOn(target) if blast_doors.contains(target) => {
                    // Blast doors and their buttons (`blast-doors.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if blast_doors.use_on(target, tool, now).is_some() {
                        sounds.tool_used(tool, mob_pos.0);
                    }
                }
                Action::UseOn(target) if floodlights.contains(target) => {
                    // Floodlights (`floodlights.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = floodlights.use_on(mob, target, tool, &mut hands, &mut commands) {
                        sounds.tool_used(tool, mob_pos.0);
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if igniters.contains(target) => {
                    // Igniters, sparkers and ignition switches (`igniters.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = igniters.use_on(target, tool, now) {
                        outbox.0.extend(outcome.to_mob.to_mob);
                        if let Some(seen) = outcome.seen {
                            sounds.tool_used(tool, mob_pos.0);
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), Some(mob)));
                        }
                    }
                }
                Action::UseOn(target) if consoles.contains(target) => {
                    // Security camera consoles open their camera list (`cameras.md`).
                    crate::cameras::open(&consoles, mob, mob_pos.0, target, &mut commands);
                }
                Action::UseOn(target) if cameras.contains(target) => {
                    // Wirecutters switch a camera off or on (`cameras.md`), within reach.
                    let Ok((at, mut camera)) = cameras.get_mut(target) else {
                        continue;
                    };
                    if !in_reach(mob_pos.0, at.0) || !holding(&hands, &appearances, "wirecutters") {
                        continue;
                    }
                    let outcome = crate::cameras::cut(&mut camera);
                    sounds.tool_used(tool, at.0);
                    if let Some(seen) = outcome.seen {
                        let name = appearances
                            .get(mob)
                            .ok()
                            .and_then(|(_, n)| n.map(|n| n.0.clone()))
                            .unwrap_or_default();
                        visible.0.push((mob_pos.0, seen.arg("name", name), None));
                    }
                }
                Action::UseOn(target) if vendors.contains(target) => {
                    // Vending machines open their window (`vending.md`).
                    crate::vending::open(&vendors, mob, mob_pos.0, target, &mut commands);
                }
                Action::UseOn(target) if walllockers.contains(target) => {
                    // Emergency wall lockers (`walllockers.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = walllockers.use_on(target, &mut hands, tool, &mut commands) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if chairs.contains(target) => {
                    // Chairs (`chairs.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = chairs.use_on(mob, mob_pos.0, target, tool, &mut commands) {
                        sounds.tool_used(tool, mob_pos.0);
                        // Whoever is told something themselves is spared the version others see.
                        let spared = outcome.to_mob.is_some().then_some(mob);
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), spared));
                        }
                    }
                }
                Action::UseOn(target) if chargers.contains(target) => {
                    // Cell chargers (`cells.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = chargers.use_on(mob, target, tool, &mut hands, &mut commands) {
                        sounds.tool_used(tool, mob_pos.0);
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), Some(mob)));
                        }
                    }
                }
                Action::UseOn(target) if dispensers.contains(target) => {
                    // Tank storage units (`tank-dispenser.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) = dispensers.use_on(mob, target, tool, &mut hands, &mut commands) {
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if generators.contains(target) => {
                    // Portable generators (`generators.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    if let Some(outcome) =
                        generators.use_on(target, tool, &mut hands, &mut cable.floors.stacks, &mut commands)
                    {
                        if outcome.to_mob.is_some() {
                            sounds.tool_used(tool, mob_pos.0);
                        }
                        outbox.0.extend(outcome.to_mob);
                    }
                }
                Action::UseOn(target) if closets.contains(target) => {
                    // Closets, lockers and crates (`closets.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    let lit = welding.lit(hands.active_item()).is_some();
                    if let Some(outcome) = closets.use_on(mob, &mut hands, lit, target, &mut commands) {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                    }
                }
                Action::UseOn(target) if lights.contains(target) => {
                    // Lights and light switches (`lights.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    let harm = intent.as_ref().is_some_and(|i| i.0 == rs13_world::Intent::Harm);
                    if let Some(outcome) =
                        lights.use_on(mob, &mut hands, harm, target, &mut shocks, &mut commands)
                    {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), Some(mob)));
                        }
                    }
                }
                Action::UseOn(target) => {
                    // Containers, their contents and things the mob carries
                    // (`docs/reference/storage.md`). A container on the floor must be in reach.
                    let reachable = targets
                        .get(target)
                        .is_ok_and(|(p, ..)| p.is_none_or(|p| in_reach(mob_pos.0, p.0)));
                    if reachable && let Some(outcome) = storage.use_on(mob, &mut hands, target, &mut commands)
                    {
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                        continue;
                    }
                    // Cable coils and wirecutters on cables, coils and power machines
                    // (`docs/reference/cables.md`).
                    if let Some(outcome) = cable.use_on(mob, mob_pos.0, &mut hands, target, &mut commands) {
                        sounds.tool_used(tool, places.get(target).map_or(mob_pos.0, |p| p.0));
                        outbox.0.extend(outcome.to_mob);
                        if let Some(seen) = outcome.seen {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            visible.0.push((mob_pos.0, seen.arg("name", name), None));
                        }
                        continue;
                    }
                    // Welding tools on fuel tanks (`welding.md`).
                    if reachable
                        && let Some(active) = hands.active_item()
                        && let Some(outcome) = welding.use_on(active, target)
                    {
                        outbox.0.extend(outcome.to_mob);
                        continue;
                    }
                    // Tables, racks and their parts (`tables.md`).
                    if reachable {
                        let at = places.get(target).map_or(mob_pos.0, |p| p.0);
                        if let Some(outcome) =
                            furniture.use_on(mob, mob_pos.0, &mut hands, target, at, &mut commands)
                        {
                            sounds.tool_used(tool, at);
                            outbox.0.extend(outcome.to_mob);
                            continue;
                        }
                    }
                    // Floor tiles onto a pile of floor tiles (`floors.md`).
                    if reachable && cable.floors.use_on(&mut hands, target, &mut commands).is_some() {
                        continue;
                    }
                    let Ok((Some(target_pos), is_item, airlock, switch, member, is_canister, is_valve)) =
                        targets.get_mut(target)
                    else {
                        // Held by someone, or gone.
                        continue;
                    };
                    if !in_reach(mob_pos.0, target_pos.0) {
                        continue;
                    }
                    if let Ok(mut door) = windoors.get_mut(target) {
                        // Window doors, by hand or with anything (`windoors.md`).
                        let shown = closets.shown(mob, &hands);
                        crate::windoors::click(&mut door, target_pos.0, mob_pos.0, shown, now, &mut sounds);
                        continue;
                    }
                    if is_item && hands.active_item().is_none() && taken.insert(target) {
                        let active = hands.active;
                        *hands.slot_mut(active) = Some(target);
                        commands.entity(target).remove::<GridPos>().insert(HeldBy(mob));
                    } else if is_canister && holding(&hands, &appearances, "wrench") {
                        // Wrench a canister, portable pump or scrubber on or off the port
                        // under it.
                        let Some(mut member) = member else { continue };
                        if member.open || ports.iter().any(|p| p.0 == target_pos.0) {
                            member.open = !member.open;
                            nets.dirty = true;
                            sounds.tool_used(tool, target_pos.0);
                            let id = if member.open {
                                "canister-connected"
                            } else {
                                "canister-disconnected"
                            };
                            let noun = appearances
                                .get(target)
                                .map_or("canister", |(a, _)| a.sprite.as_str());
                            outbox.0.push(LocMsg::new(id).arg("item", noun));
                        }
                    } else if is_canister {
                        // Anything else opens its window; a gas tank goes in
                        // (`portables.md`).
                        gas_windows.use_on(mob, target, &mut hands, &mut commands);
                    } else if let Ok((mut alarm, power)) = alarms.get_mut(target) {
                        // Luna `attackby`: wirecutters disconnect or reconnect it; anything
                        // else opens the window, if it works.
                        if holding(&hands, &appearances, "wirecutters") {
                            let name = appearances
                                .get(mob)
                                .ok()
                                .and_then(|(_, n)| n.map(|n| n.0.clone()))
                                .unwrap_or_default();
                            let msg = crate::alarms::cut_alarm(&mut alarm, &name);
                            sounds.tool_used(tool, target_pos.0);
                            visible.0.push((target_pos.0, msg, None));
                        } else if !alarm.broken && power.powered {
                            commands.entity(mob).insert(crate::wires::PanelViewer(target));
                        }
                    } else if let Some(mut switch) = switch {
                        switch.0 = !switch.0;
                        let sprite = appearances
                            .get(target)
                            .map(|(a, _)| a.sprite.clone())
                            .unwrap_or_default();
                        let id = match (is_valve, switch.0) {
                            (true, true) => "valve-opened",
                            (true, false) => "valve-closed",
                            (false, true) => "switched-on",
                            (false, false) => "switched-off",
                        };
                        if is_valve && let Some(mut member) = member {
                            member.open = switch.0;
                            nets.dirty = true;
                        }
                        outbox.0.push(LocMsg::new(id).arg("item", sprite));
                    } else if let Some(mut airlock) = airlock {
                        // Luna `attackby` / `attack_hand`: an electrified door shocks first.
                        if electrified.get(target).is_ok_and(|w| w.is_electrified()) && airlock.powered {
                            let chance = if hands.active_item().is_some() {
                                crate::wires::TOOL_SHOCK
                            } else {
                                crate::wires::HAND_SHOCK
                            };
                            shocks.0.push((mob, chance));
                            continue;
                        }
                        // A lit welding tool welds a closed door shut, or open again.
                        if let Some(active) = hands.active_item()
                            && welding.lit(Some(active)).is_some()
                            && airlock.state == DoorState::Closed
                        {
                            if welding.weld(active, crate::welding::DOOR_FUEL) {
                                airlock.welded = !airlock.welded;
                            } else {
                                outbox.0.push(LocMsg::new("welder-short"));
                            }
                            continue;
                        }
                        if holding(&hands, &appearances, "screwdriver") {
                            airlock.panel_open = !airlock.panel_open;
                            sounds.tool_used(tool, target_pos.0);
                            let id = if airlock.panel_open {
                                "door-panel-opened"
                            } else {
                                "door-panel-closed"
                            };
                            outbox.0.push(LocMsg::new(id).arg("item", "airlock"));
                            continue;
                        }
                        if holding(&hands, &appearances, "crowbar") && airlock.force(now) {
                            sounds.tool_used(tool, target_pos.0);
                            outbox.0.push(LocMsg::new("door-forced").arg("item", "airlock"));
                            continue;
                        }
                        // With the panel open, using the door shows its wires instead.
                        if airlock.panel_open {
                            commands.entity(mob).insert(crate::wires::PanelViewer(target));
                            continue;
                        }
                        // By hand or with any item (`access.md`).
                        airlock.use_by(now, closets.shown(mob, &hands));
                    }
                }
            }
        }
    }
    for (mob, welder, at) in welded_with {
        // Luna plays `Welder.ogg` or `Welder2.ogg` at 50 when welding.
        sounds.play(crate::sound::WELDER, at, 50.0);
        if let Some(msg) = welding.after_use(welder)
            && let Ok((.., mut outbox, _, _, _)) = mobs.get_mut(mob)
        {
            outbox.0.push(msg);
        }
    }
}

/// Whether the active hand holds an item with this sprite id.
fn holding(hands: &Hands, appearances: &Query<(&Appearance, Option<&DisplayName>)>, sprite: &str) -> bool {
    hands
        .active_item()
        .and_then(|e| appearances.get(e).ok())
        .is_some_and(|(a, _)| a.sprite == sprite)
}

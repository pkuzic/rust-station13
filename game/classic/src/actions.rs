//! Player and NPC actions other than movement: clicking things, dropping, swapping
//! hands, examining. Spec: `docs/reference/hands.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, DisplayName, GridPos, Hidden, Intent, MobStat, TilePos};

use crate::access::IdCard;
use crate::combat::{CombatIntent, PendingAttacks};
use crate::doors::{Airlock, DoorState};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals};
use crate::machines::{Canister, Connector, GasStore, PipeMember, PipeNets, Switch, Valve};
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
            Has<Canister>,
            Has<Valve>,
        ),
        Without<Mob>,
    >,
    ports: Query<&GridPos, (With<Connector>, Without<Mob>)>,
    stores: Query<&GasStore>,
    mut nets: ResMut<PipeNets>,
    batteries: Query<&Smes>,
    cards: Query<&IdCard>,
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
        if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
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
                Action::SwapHands => hands.active = hands.active.other(),
                // Turn on the spot; a chair turns its sitter back (`movement::sync_facing`).
                Action::Face(dir) => mob_state.facing = Some(dir),
                Action::Pull(target) => pulls.0.push((mob, target)),
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
                        if let Some(more) = cable.examine(target) {
                            outbox.0.push(more);
                        }
                        if let Some(more) = cable.floors.examine(target) {
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
                            || radios.contains(target)) =>
                {
                    // The item in the active hand used on itself (Luna `attack_self`): a
                    // welding tool lights or goes out, rack parts become a rack, a
                    // flashlight switches.
                    if let Some(outcome) = welding
                        .use_self(target)
                        .or_else(|| flashlights.use_self(target))
                        .or_else(|| jetpacks.use_self(target))
                        .or_else(|| radios.toggle_microphone(target))
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
                Action::UseOn(target) if igniters.contains(target) => {
                    // Igniters and ignition switches (`igniters.md`), within reach.
                    if !places.get(target).is_ok_and(|p| in_reach(mob_pos.0, p.0)) {
                        continue;
                    }
                    igniters.use_on(target, now);
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
                    if is_item && hands.active_item().is_none() && taken.insert(target) {
                        let active = hands.active;
                        *hands.slot_mut(active) = Some(target);
                        commands.entity(target).remove::<GridPos>().insert(HeldBy(mob));
                    } else if is_canister && holding(&hands, &appearances, "wrench") {
                        // Wrench a canister on or off the port under it.
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
                        // By hand or with any item; only an ID card in the active hand
                        // counts for access.
                        let card = hands.active_item().and_then(|e| cards.get(e).ok());
                        airlock.use_by(now, card);
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

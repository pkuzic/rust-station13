//! Admins, bans and adminhelp, as Luna has them (`modules/admin`, `NewBan`): a list of keys and ranks on
//! the server, and the verbs each rank gets. BYOND gives Luna its keys. Here each client makes itself a
//! secret and the key is a hash of it. Spec: `docs/reference/admin.md`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use rs13_classic::combat::{Godmode, Muted};
use rs13_classic::health::{Body, Vitals};
use rs13_classic::objects::spawn_object;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{Notice, NoticeChannel};
use rs13_world::{DisplayName, Ghost, GridPos, MobStat, ObjectSpec, TilePos};
use sha2::{Digest, Sha256};

use crate::Controls;
use crate::lobby::InLobby;

/// Where the admins and bans are kept, under the server's working directory.
pub const ADMINS_FILE: &str = "config/admins.txt";
pub const BANS_FILE: &str = "data/bans.txt";
/// How long someone being kicked has to get the message, ds of real time in ticks.
const KICK_TICKS: u32 = 20;
/// Longest secret a client may send.
const MAX_SECRET: usize = 256;

/// Luna's ranks, as levels (`update_admins`).
pub fn level_of(rank: &str) -> Option<i32> {
    Some(match rank.trim() {
        "Host" => 6,
        "Coder" => 5,
        "Super Administrator" => 4,
        "Primary Administrator" => 3,
        "Administrator" => 2,
        "Secondary Administrator" => 1,
        "Moderator" => 0,
        _ => return None,
    })
}

/// A client's key, from its secret.
pub fn key_of(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    let mut key = String::new();
    for byte in digest.iter().take(8) {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ban {
    pub key: String,
    pub address: Option<IpAddr>,
    /// Unix seconds it ends at, or `None` for good.
    pub until: Option<u64>,
    pub by: String,
    pub reason: String,
}

/// Who's an admin, who's banned, and whether OOC is on.
#[derive(Resource, Debug, Clone)]
pub struct AdminConfig {
    pub admins: HashMap<String, i32>,
    pub bans: Vec<Ban>,
    /// Where bans are saved. `None` keeps them in memory.
    pub bans_file: Option<PathBuf>,
    pub ooc_on: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        let admins =
            std::fs::read_to_string(ADMINS_FILE).map_or_else(|_| HashMap::new(), |t| parse_admins(&t));
        let bans = std::fs::read_to_string(BANS_FILE).map_or_else(|_| Vec::new(), |t| parse_bans(&t));
        Self {
            admins,
            bans,
            bans_file: Some(PathBuf::from(BANS_FILE)),
            ooc_on: true,
        }
    }
}

/// Luna's admins file: `key - rank` on each line, `;` for a comment.
pub fn parse_admins(text: &str) -> HashMap<String, i32> {
    let mut admins = HashMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let Some((key, rank)) = line.split_once(" - ") else {
            warn!(line, "not an admin line");
            continue;
        };
        match level_of(rank) {
            Some(level) => {
                admins.insert(key.trim().to_lowercase(), level);
            }
            None => warn!(rank, "no such rank"),
        }
    }
    admins
}

/// One ban to a line: key, address, end, who by, and the reason, split by `|`.
pub fn parse_bans(text: &str) -> Vec<Ban> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(5, '|');
            Some(Ban {
                key: parts.next()?.to_owned(),
                address: parts.next()?.parse().ok(),
                until: parts.next()?.parse().ok(),
                by: parts.next()?.to_owned(),
                reason: parts.next()?.to_owned(),
            })
        })
        .collect()
}

fn write_bans(bans: &[Ban]) -> String {
    bans.iter().fold(String::new(), |mut out, ban| {
        let address = ban.address.map_or_else(String::new, |a| a.to_string());
        let until = ban.until.map_or_else(String::new, |u| u.to_string());
        let _ = writeln!(
            out,
            "{}|{address}|{until}|{}|{}",
            ban.key,
            ban.by,
            ban.reason.replace('\n', " ")
        );
        out
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl AdminConfig {
    /// The ban that keeps this key or address out, if any is still running.
    pub fn banned(&self, key: &str, address: Option<IpAddr>) -> Option<&Ban> {
        let now = unix_now();
        self.bans
            .iter()
            .filter(|ban| ban.until.is_none_or(|until| until > now))
            .find(|ban| ban.key == key || (address.is_some() && ban.address == address))
    }

    fn save(&self) {
        let Some(path) = &self.bans_file else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(path, write_bans(&self.bans)) {
            warn!(%error, "couldn't save the bans");
        }
    }
}

/// What a client asks of the admin system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminInput {
    /// Who it is.
    Hello(String),
    Help(String),
    Command(String),
}

/// What came in this tick, by link.
#[derive(Resource, Debug, Default)]
pub struct AdminInbox(pub Vec<(Entity, AdminInput)>);

/// On a link: who it is.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PlayerKey(pub String);

/// On a link: an admin, at this level.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminLevel(pub i32);

/// On a link: to be dropped after this many ticks, once it has had time to hear why.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kicked(pub u32);

fn tell(world: &mut World, link: Entity, msg: LocMsg) {
    if let Some(mut sender) = world.get_mut::<MessageSender<Notice>>(link) {
        sender.send::<NoticeChannel>(Notice(msg));
    }
}

fn tell_admins(world: &mut World, msg: &LocMsg) {
    let mut admins = world.query_filtered::<&mut MessageSender<Notice>, With<AdminLevel>>();
    for mut sender in admins.iter_mut(world) {
        sender.send::<NoticeChannel>(Notice(msg.clone()));
    }
}

fn tell_everyone(world: &mut World, msg: &LocMsg) {
    let mut links = world.query_filtered::<&mut MessageSender<Notice>, With<RemoteId>>();
    for mut sender in links.iter_mut(world) {
        sender.send::<NoticeChannel>(Notice(msg.clone()));
    }
}

fn address_of(world: &World, link: Entity) -> Option<IpAddr> {
    world.get::<PeerAddr>(link).map(|addr| addr.0.ip())
}

fn name_of_link(world: &World, link: Entity) -> Option<String> {
    let of_mob = world
        .get::<Controls>(link)
        .and_then(|controls| world.get::<DisplayName>(controls.0))
        .map(|name| name.0.clone());
    of_mob.or_else(|| world.get::<InLobby>(link).and_then(|lobby| lobby.name.clone()))
}

fn who_said(world: &World, link: Entity) -> String {
    name_of_link(world, link)
        .or_else(|| world.get::<PlayerKey>(link).map(|key| key.0.clone()))
        .unwrap_or_default()
}

fn kick(world: &mut World, link: Entity, msg: LocMsg) {
    tell(world, link, msg);
    world.entity_mut(link).insert(Kicked(KICK_TICKS));
}

/// The link of whoever goes by this name or key, the name's start being enough.
fn find_link(world: &mut World, wanted: &str) -> Option<Entity> {
    let wanted = wanted.to_lowercase();
    let mut links = world.query_filtered::<Entity, With<RemoteId>>();
    let all: Vec<Entity> = links.iter(world).collect();
    all.into_iter().find(|link| {
        world.get::<PlayerKey>(*link).is_some_and(|key| key.0 == wanted)
            || name_of_link(world, *link).is_some_and(|name| name.to_lowercase().starts_with(&wanted))
    })
}

fn no_one(world: &mut World, link: Entity, who: &str) {
    tell(world, link, LocMsg::new("admin-no-one").arg("name", who));
}

/// May this link do this to that one? No one acts on an admin of their own rank or higher.
fn outranks(world: &World, link: Entity, target: Entity) -> bool {
    let own = world.get::<AdminLevel>(link).map_or(-1, |a| a.0);
    world.get::<AdminLevel>(target).is_none_or(|a| a.0 < own)
}

/// The lowest level each verb needs (Luna's `averbs_*`).
fn needs(verb: &str) -> Option<i32> {
    Some(match verb {
        "help" | "who" | "asay" | "pm" | "dsay" | "announce" | "startnow" | "toggleooc" | "jump"
        | "jumpto" | "kick" | "ban" | "unban" | "bans" => 0,
        "restart" => 1,
        "mute" | "explode" | "emp" => 2,
        "get" | "spawn" | "god" | "rejuv" | "delete" => 4,
        _ => return None,
    })
}

fn mob_at(world: &World, mob: Entity) -> Option<TilePos> {
    world.get::<GridPos>(mob).map(|p| p.0)
}

fn own_mob(world: &World, link: Entity) -> Option<Entity> {
    world.get::<Controls>(link).map(|c| c.0)
}

fn run_command(world: &mut World, link: Entity, line: &str) {
    let line = line.trim();
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let verb = verb.to_lowercase();
    let rest = rest.trim();
    // Anyone can ask who they are: it's how a new admin finds their key.
    if verb == "key" {
        let key = world
            .get::<PlayerKey>(link)
            .map_or_else(String::new, |k| k.0.clone());
        tell(world, link, LocMsg::new("admin-your-key").arg("key", key));
        return;
    }
    let level = world.get::<AdminLevel>(link).map(|a| a.0);
    let Some(level) = level else {
        tell(world, link, LocMsg::new("admin-not-admin"));
        return;
    };
    let Some(wants) = needs(&verb) else {
        tell(world, link, LocMsg::new("admin-unknown").arg("verb", verb));
        return;
    };
    if level < wants {
        tell(world, link, LocMsg::new("admin-low-rank"));
        return;
    }
    let by = who_said(world, link);
    info!(by, line, "admin");
    let (first, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    let tail = tail.trim();
    match verb.as_str() {
        "help" => tell(world, link, LocMsg::new("admin-help")),
        "who" => who(world, link),
        "asay" => {
            let msg = LocMsg::new("admin-asay").arg("name", by).arg("text", rest);
            tell_admins(world, &msg);
        }
        "dsay" => {
            let msg = LocMsg::new("ghost-dsay").arg("name", by).arg("text", rest);
            tell_admins(world, &msg);
            let mut links = world.query::<(Entity, &Controls)>();
            let all: Vec<(Entity, Entity)> = links.iter(world).map(|(l, c)| (l, c.0)).collect();
            for (other, mob) in all {
                let dead = world.get::<Ghost>(mob).is_some()
                    || world.get::<Vitals>(mob).is_some_and(|v| v.stat == MobStat::Dead);
                if dead && world.get::<AdminLevel>(other).is_none() {
                    tell(world, other, msg.clone());
                }
            }
        }
        "pm" => {
            let Some(target) = find_link(world, first) else {
                return no_one(world, link, first);
            };
            tell(
                world,
                target,
                LocMsg::new("admin-pm-from")
                    .arg("name", by.clone())
                    .arg("text", tail),
            );
            let msg = LocMsg::new("admin-pm-to")
                .arg("name", by)
                .arg("target", who_said(world, target))
                .arg("text", tail);
            tell_admins(world, &msg);
        }
        "announce" => {
            let msg = LocMsg::new("admin-announce").arg("text", rest);
            tell_everyone(world, &msg);
        }
        "startnow" => {
            if let Some(mut round) = world.get_resource_mut::<rs13_classic::round::Round>()
                && matches!(round.phase, rs13_classic::round::Phase::Pregame { .. })
            {
                round.phase = rs13_classic::round::Phase::Pregame { starts_at: 0.0 };
            }
            let msg = LocMsg::new("admin-started").arg("name", by);
            tell_admins(world, &msg);
        }
        "restart" => {
            world.write_message(AppExit::from_code(crate::RESTART_CODE));
        }
        "toggleooc" => {
            let mut config = world.resource_mut::<AdminConfig>();
            config.ooc_on = !config.ooc_on;
            let on = config.ooc_on;
            let msg = LocMsg::new(if on { "admin-ooc-on" } else { "admin-ooc-off" });
            tell_everyone(world, &msg);
        }
        "kick" => {
            let Some(target) = find_link(world, first) else {
                return no_one(world, link, first);
            };
            if !outranks(world, link, target) {
                return tell(world, link, LocMsg::new("admin-low-rank"));
            }
            let msg = LocMsg::new("admin-kicked").arg("name", by).arg("text", tail);
            kick(world, target, msg);
        }
        "ban" => ban(world, link, first, tail, by),
        "unban" => {
            let mut config = world.resource_mut::<AdminConfig>();
            let before = config.bans.len();
            config.bans.retain(|ban| ban.key != first);
            let removed = config.bans.len() != before;
            config.save();
            let id = if removed { "admin-unbanned" } else { "admin-no-ban" };
            tell(world, link, LocMsg::new(id).arg("key", first));
        }
        "bans" => {
            let now = unix_now();
            let bans = world.resource::<AdminConfig>().bans.clone();
            for ban in bans {
                let minutes = ban
                    .until
                    .map_or(-1, |until| (until.saturating_sub(now) / 60) as i64);
                let msg = LocMsg::new("admin-ban-row")
                    .arg("key", ban.key)
                    .arg("by", ban.by)
                    .num("value", minutes)
                    .arg("text", ban.reason);
                tell(world, link, msg);
            }
        }
        "mute" => {
            let Some(target) = find_link(world, first) else {
                return no_one(world, link, first);
            };
            if !outranks(world, link, target) {
                return tell(world, link, LocMsg::new("admin-low-rank"));
            }
            let muting = world.get::<Muted>(target).is_none();
            let mob = own_mob(world, target);
            for thing in [Some(target), mob].into_iter().flatten() {
                if muting {
                    world.entity_mut(thing).insert(Muted);
                } else {
                    world.entity_mut(thing).remove::<Muted>();
                }
            }
            tell(
                world,
                target,
                LocMsg::new(if muting { "admin-muted" } else { "admin-voiced" }),
            );
            let msg = LocMsg::new("admin-did")
                .arg("name", by)
                .arg("verb", verb)
                .arg("target", first);
            tell_admins(world, &msg);
        }
        "emp" => {
            let numbers: Vec<i32> = rest.split_whitespace().filter_map(|n| n.parse().ok()).collect();
            if let (Some(here), [heavy, light, ..]) = (
                own_mob(world, link).and_then(|mob| mob_at(world, mob)),
                numbers.as_slice(),
            ) {
                rs13_classic::explosions::empulse(world, here, *heavy, *light);
            }
        }
        "jump" | "explode" => {
            let numbers: Vec<i32> = rest.split_whitespace().filter_map(|n| n.parse().ok()).collect();
            let Some(mob) = own_mob(world, link) else {
                return;
            };
            let here = mob_at(world, mob);
            if verb == "jump" {
                let (Some(here), [x, y, ..]) = (here, numbers.as_slice()) else {
                    return tell(world, link, LocMsg::new("admin-help"));
                };
                let z = numbers.get(2).copied().unwrap_or(here.z);
                world.entity_mut(mob).insert(GridPos(TilePos::new(*x, *y, z)));
            } else if let (Some(here), [size, ..]) = (here, numbers.as_slice()) {
                // Luna's own `cmd_explode_turf`.
                let size = *size;
                rs13_classic::explosions::explode(world, here, size, size * 3 / 2, size * 2, size * 3);
            }
        }
        "jumpto" | "get" => {
            let Some(target) = find_link(world, first).and_then(|t| own_mob(world, t)) else {
                return no_one(world, link, first);
            };
            let Some(mob) = own_mob(world, link) else {
                return;
            };
            let (moved, to) = if verb == "jumpto" {
                (mob, target)
            } else {
                (target, mob)
            };
            if let Some(at) = mob_at(world, to) {
                world.entity_mut(moved).insert(GridPos(at));
            }
        }
        "spawn" => {
            let Some(at) = own_mob(world, link).and_then(|mob| mob_at(world, mob)) else {
                return;
            };
            if let Err(problem) = spawn_object(world, &ObjectSpec::new(first), at) {
                warn!(problem, "admin spawn");
                tell(world, link, LocMsg::new("admin-no-thing").arg("name", first));
            }
        }
        "delete" => {
            let at = own_mob(world, link).and_then(|mob| mob_at(world, mob));
            let mut things = world.query_filtered::<(Entity, &GridPos, &rs13_world::Appearance), Without<rs13_classic::movement::Mob>>();
            let found = things
                .iter(world)
                .find(|(_, p, look)| Some(p.0) == at && look.sprite == first)
                .map(|(e, ..)| e);
            match found {
                Some(thing) => world.entity_mut(thing).despawn(),
                None => tell(world, link, LocMsg::new("admin-no-thing").arg("name", first)),
            }
        }
        "god" | "rejuv" => {
            let Some(target) = find_link(world, first).and_then(|t| own_mob(world, t)) else {
                return no_one(world, link, first);
            };
            if verb == "god" {
                if world.get::<Godmode>(target).is_some() {
                    world.entity_mut(target).remove::<Godmode>();
                } else {
                    world.entity_mut(target).insert(Godmode);
                }
            } else {
                world.entity_mut(target).insert(Body::default());
                if let Some(mut vitals) = world.get_mut::<Vitals>(target) {
                    vitals.stat = MobStat::Conscious;
                }
            }
            let msg = LocMsg::new("admin-did")
                .arg("name", by)
                .arg("verb", verb)
                .arg("target", first);
            tell_admins(world, &msg);
        }
        _ => {}
    }
}

fn who(world: &mut World, link: Entity) {
    let mut links = world.query_filtered::<Entity, With<RemoteId>>();
    let all: Vec<Entity> = links.iter(world).collect();
    for other in all {
        let key = world
            .get::<PlayerKey>(other)
            .map_or_else(String::new, |k| k.0.clone());
        let name = name_of_link(world, other).unwrap_or_default();
        let job = own_mob(world, other)
            .and_then(|mob| world.get::<rs13_classic::jobs::HasJob>(mob))
            .map_or_else(|| "none".to_owned(), |job| job.0.clone());
        let address = address_of(world, other).map_or_else(String::new, |a| a.to_string());
        let msg = LocMsg::new("admin-who-row")
            .arg("name", name)
            .arg("key", key)
            .arg("job", job)
            .arg("address", address);
        tell(world, link, msg);
    }
}

/// `ban who minutes reason`, with 0 minutes for good (Luna's `newban`, and `Manual Ban` for someone not
/// here: a key that matches nobody is banned as it stands).
fn ban(world: &mut World, link: Entity, who_arg: &str, tail: &str, by: String) {
    let (minutes, reason) = tail.split_once(' ').unwrap_or((tail, ""));
    let Ok(minutes) = minutes.parse::<u64>() else {
        return tell(world, link, LocMsg::new("admin-help"));
    };
    let until = (minutes > 0).then(|| unix_now() + minutes * 60);
    let target = find_link(world, who_arg);
    if let Some(target) = target
        && !outranks(world, link, target)
    {
        return tell(world, link, LocMsg::new("admin-low-rank"));
    }
    let key = target
        .and_then(|t| world.get::<PlayerKey>(t).map(|k| k.0.clone()))
        .unwrap_or_else(|| who_arg.to_lowercase());
    let address = target.and_then(|t| address_of(world, t));
    let mut config = world.resource_mut::<AdminConfig>();
    config.bans.retain(|ban| ban.key != key);
    config.bans.push(Ban {
        key: key.clone(),
        address,
        until,
        by: by.clone(),
        reason: reason.to_owned(),
    });
    config.save();
    let msg = LocMsg::new("admin-did")
        .arg("name", by.clone())
        .arg("verb", "ban")
        .arg("target", key);
    tell_admins(world, &msg);
    if let Some(target) = target {
        let told = LocMsg::new("admin-banned").arg("name", by).arg("text", reason);
        kick(world, target, told);
    }
}

fn hello(world: &mut World, link: Entity, secret: &str) {
    if secret.is_empty() || secret.len() > MAX_SECRET || world.get::<PlayerKey>(link).is_some() {
        return;
    }
    let key = key_of(secret);
    let address = address_of(world, link);
    info!(key, ?address, "client identified");
    let config = world.resource::<AdminConfig>();
    if let Some(ban) = config.banned(&key, address) {
        let msg = LocMsg::new("admin-banned")
            .arg("name", ban.by.clone())
            .arg("text", ban.reason.clone());
        world.entity_mut(link).insert(PlayerKey(key));
        // They never really arrived: no body is left behind (Luna turns them away at the door).
        if let Some(mob) = own_mob(world, link) {
            world.entity_mut(link).remove::<Controls>();
            world.entity_mut(mob).despawn();
        }
        kick(world, link, msg);
        return;
    }
    let level = config.admins.get(&key).copied();
    world.entity_mut(link).insert(PlayerKey(key));
    if let Some(level) = level {
        world.entity_mut(link).insert(AdminLevel(level));
        tell(world, link, LocMsg::new("admin-welcome"));
    }
}

/// Requests in, and those kicked out.
pub fn run_admin(world: &mut World) {
    let asked = std::mem::take(&mut world.resource_mut::<AdminInbox>().0);
    for (link, input) in asked {
        match input {
            AdminInput::Hello(secret) => hello(world, link, &secret),
            AdminInput::Command(line) => run_command(world, link, &line),
            AdminInput::Help(text) => {
                let text = text.trim();
                if text.is_empty() || world.get::<Muted>(link).is_some() {
                    continue;
                }
                let name = who_said(world, link);
                let key = world
                    .get::<PlayerKey>(link)
                    .map_or_else(String::new, |k| k.0.clone());
                let msg = LocMsg::new("admin-help-from")
                    .arg("name", name)
                    .arg("key", key)
                    .arg("text", text);
                tell_admins(world, &msg);
                tell(world, link, LocMsg::new("admin-help-sent").arg("text", text));
            }
        }
    }
    let mut kicked = world.query::<(Entity, &mut Kicked)>();
    let mut gone = Vec::new();
    for (link, mut left) in kicked.iter_mut(world) {
        left.0 = left.0.saturating_sub(1);
        if left.0 == 0 {
            gone.push(link);
        }
    }
    for link in gone {
        world.entity_mut(link).despawn();
    }
}

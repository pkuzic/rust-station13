//! Scripted steps for a bot, so tests can drive the real game over the network: walk,
//! click things, act, wait, and note what the bot can see at points along the way.

use std::time::Duration;

use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, MoveInput, Player, StatusUpdate};
use rs13_world::{Appearance, Dir, DisplayName, GridPos, Hand, HandsView, NetId, Silhouette, TilePos};

use crate::{JoinedAt, SeenPlayer};

/// One step of a bot's script, run in order once its own player has arrived.
#[derive(Debug, Clone)]
pub enum Step {
    /// Hold a direction; `None` stops.
    Move(Option<Dir>),
    /// Click, with the active hand, the first thing with this sprite the bot has been sent
    /// (not its own mob). Waits until there is one.
    UseOn(String),
    /// Ctrl-click: start pulling the first thing with this sprite. Waits until there is one.
    Pull(String),
    /// Pick line `N` of the open list window of the first thing with this sprite (not its own
    /// mob). Waits until there is one.
    PickOn(String, u16),
    /// Click the item in the active hand (Luna `attack_self`: light a welding tool, flip
    /// an extinguisher's safety). Held items aren't sent as things on the map, so `UseOn`
    /// can't find them. Waits until the hand holds something.
    UseHeld,
    /// Cast line `N` of the bot's spells at the first thing with this sprite it has been sent
    /// (not its own mob). Waits until there is one.
    CastOn(u8, String),
    /// Send any other action.
    Act(ActionInput),
    Wait(Duration),
    /// Note what the bot can see right now, under this label.
    Snapshot(String),
}

/// What the bot could see at a [`Step::Snapshot`].
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub label: String,
    pub players: Vec<SeenPlayer>,
    /// Where each thing on the map the bot has been sent is, by sprite, sorted.
    pub placed: Vec<(String, TilePos)>,
    /// The last status the server sent.
    pub status: Option<StatusUpdate>,
    /// The sprite of every thing the bot has been sent, sorted.
    pub sprites: Vec<String>,
    /// Where each silhouette (someone in the dark) the bot has been sent stands, sorted.
    pub silhouettes: Vec<TilePos>,
    /// Every name the bot has been sent, sorted.
    pub names: Vec<String>,
}

#[derive(Resource, Default)]
pub(crate) struct Snapshots(pub Vec<Snapshot>);

/// Where the bot is in its script, and when the current step started.
#[derive(Default)]
pub(crate) struct Progress {
    next: usize,
    started: Option<Duration>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn run_script(
    time: Res<Time>,
    config: Res<crate::BotConfig>,
    joined: Res<JoinedAt>,
    mut senders: Query<
        (&mut MessageSender<MoveInput>, &mut MessageSender<ActionInput>),
        (With<Client>, With<Connected>),
    >,
    things: Query<(&NetId, &Appearance, Has<Controlled>, Option<&GridPos>)>,
    received: Res<crate::Received>,
    players: Query<(&Player, &GridPos, Has<Controlled>)>,
    (shapes, names, hands): (
        Query<&GridPos, With<Silhouette>>,
        Query<&DisplayName>,
        Query<&HandsView, With<Controlled>>,
    ),
    mut snapshots: ResMut<Snapshots>,
    mut progress: Local<Progress>,
) {
    if joined.0.is_none() {
        return;
    }
    let Ok((mut moves, mut actions)) = senders.single_mut() else {
        return;
    };
    let now = time.elapsed();
    let Some(step) = config.script.get(progress.next) else {
        return;
    };
    let started = *progress.started.get_or_insert(now);
    let find = |sprite: &str| {
        things
            .iter()
            .find(|(_, a, own, _)| !own && a.sprite == sprite)
            .map(|(id, ..)| *id)
    };
    let done = match step {
        Step::Move(dir) => {
            moves.send::<IntentChannel>(MoveInput { dir: *dir, run: true });
            true
        }
        Step::UseOn(sprite) => {
            let target = find(sprite);
            if let Some(id) = target {
                actions.send::<IntentChannel>(ActionInput::UseOn(id));
            }
            target.is_some()
        }
        Step::PickOn(sprite, line) => {
            let target = find(sprite);
            if let Some(id) = target {
                actions.send::<IntentChannel>(ActionInput::Pick {
                    target: id,
                    line: *line,
                });
            }
            target.is_some()
        }
        Step::Pull(sprite) => {
            let target = find(sprite);
            if let Some(id) = target {
                actions.send::<IntentChannel>(ActionInput::Pull(id));
            }
            target.is_some()
        }
        Step::UseHeld => {
            let held = hands.single().ok().and_then(|h| match h.active {
                Hand::Left => h.left,
                Hand::Right => h.right,
            });
            if let Some(id) = held {
                actions.send::<IntentChannel>(ActionInput::UseOn(id));
            }
            held.is_some()
        }
        Step::CastOn(line, sprite) => {
            let target = find(sprite);
            if let Some(id) = target {
                actions.send::<IntentChannel>(ActionInput::Spell(
                    rs13_network::protocol::SpellCommand::Cast {
                        spell: *line,
                        target: Some(id),
                        area: None,
                    },
                ));
            }
            target.is_some()
        }
        Step::Act(action) => {
            actions.send::<IntentChannel>(action.clone());
            true
        }
        Step::Wait(wait) => now >= started + *wait,
        Step::Snapshot(label) => {
            let mut seen: Vec<SeenPlayer> = players
                .iter()
                .map(|(player, pos, own)| SeenPlayer {
                    client_id: player.client_id,
                    pos: pos.0,
                    own,
                })
                .collect();
            seen.sort();
            let mut sprites: Vec<String> = things.iter().map(|(_, a, ..)| a.sprite.clone()).collect();
            sprites.sort();
            let mut placed: Vec<(String, TilePos)> = things
                .iter()
                .filter_map(|(_, a, _, pos)| pos.map(|p| (a.sprite.clone(), p.0)))
                .collect();
            placed.sort();
            let mut silhouettes: Vec<TilePos> = shapes.iter().map(|p| p.0).collect();
            silhouettes.sort();
            let mut names: Vec<String> = names.iter().map(|n| n.0.clone()).collect();
            names.sort();
            snapshots.0.push(Snapshot {
                label: label.clone(),
                players: seen,
                placed,
                status: received.1.clone(),
                sprites,
                silhouettes,
                names,
            });
            true
        }
    };
    if done {
        progress.next += 1;
        progress.started = None;
    }
}

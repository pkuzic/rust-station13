//! Dead players walk on as a see-through copy of their body. Spec: `docs/reference/ghosts.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, Facing, Ghost, GridPos, HandsView, MobStat, TilePos, WornView};

use crate::actions::{Outbox, PendingActions};
use crate::equipment::Equipment;
use crate::health::Vitals;
use crate::movement::MoveIntent;
use crate::multiz::{Ladder, Stairs, other_deck};
use crate::objects::{NetIds, PlayerControlled};
use crate::sound::Earshot;
use crate::speech::{PendingSpeech, sanitize_speech, speech_verb};
use crate::time::WorldTime;

/// Delay between a ghost's steps, in deciseconds: twice a person's run or so.
pub const GHOST_STEP_DS: f32 = 1.0;

/// On a ghost: the earliest world time (deciseconds) of its next step.
#[derive(Component, Debug, Default)]
pub struct GhostSteps {
    next_move_ds: f32,
}

/// On a body its player has left: their ghost.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeftBody(pub Entity);

/// Bodies whose player has just left, and the ghost they left as, for whatever connects
/// players to mobs to follow: `(body, ghost)`.
#[derive(Resource, Debug, Default)]
pub struct Departures(pub Vec<(Entity, Entity)>);

/// The player leaves `body` as a ghost where it lies, looking as the body did: standing,
/// facing the same way, in what it wore. Nothing happens if they already left it.
pub fn leave_body(world: &mut World, body: Entity) -> Option<Entity> {
    if world.get::<LeftBody>(body).is_some() {
        return None;
    }
    let at = world.get::<GridPos>(body).copied().or_else(|| {
        let closet = world.get::<crate::closets::InCloset>(body)?.0;
        world.get::<GridPos>(closet).copied()
    })?;
    let name = world.get::<DisplayName>(body)?.clone();
    let facing = world.get::<Facing>(body).copied().unwrap_or(Facing(0));
    let sprite = world.get::<Appearance>(body)?.sprite.clone();
    let worn: Vec<_> = world
        .get::<Equipment>(body)
        .map(|e| e.0.iter().map(|(slot, item)| (*slot, *item)).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(slot, item)| Some((slot, world.get::<Appearance>(item)?.clone())))
        .collect();
    let id = world.resource_mut::<NetIds>().alloc();
    let ghost = world
        .spawn((
            (
                Ghost { worn },
                at,
                id,
                Appearance::new(sprite, ""),
                facing,
                name,
                HandsView::default(),
                WornView::default(),
            ),
            (
                GhostSteps::default(),
                MoveIntent::default(),
                PendingSpeech::default(),
                PendingActions::default(),
                Outbox(vec![LocMsg::new("ghost-left-body")]),
                Earshot::default(),
            ),
        ))
        .id();
    world.entity_mut(body).insert(LeftBody(ghost));
    world.resource_mut::<Departures>().0.push((body, ghost));
    Some(ghost)
}

/// Whoever has just died leaves their body. Death is permanent (`health.md`), so they
/// don't have to ask.
pub fn leave_dead_bodies(world: &mut World) {
    let dead: Vec<Entity> = world
        .query_filtered::<(Entity, &Vitals), (With<PlayerControlled>, Without<LeftBody>)>()
        .iter(world)
        .filter(|(_, vitals)| vitals.stat == MobStat::Dead)
        .map(|(body, _)| body)
        .collect();
    for body in dead {
        leave_body(world, body);
    }
}

/// Ghosts go where they are asked to: through walls and doors, over open space, but not
/// off the deck.
pub fn move_ghosts(
    time: Res<WorldTime>,
    decks: crate::decks::Decks,
    mut ghosts: Query<(&mut GridPos, &mut GhostSteps, &MoveIntent, &mut Facing), With<Ghost>>,
) {
    let now = time.deciseconds();
    for (mut pos, mut steps, intent, mut facing) in &mut ghosts {
        let Some(dir) = intent.dir else { continue };
        if now < steps.next_move_ds {
            continue;
        }
        facing.set_if_neq(Facing::of(dir));
        let to = pos.0.step(dir);
        if decks.tile(to).is_some() {
            pos.0 = to;
            steps.next_move_ds = now + GHOST_STEP_DS;
        }
    }
}

/// What ghosts do with a click, any distance away: a ladder or stairs take them to the same
/// spot on the deck they lead to; anything else they look at.
#[allow(clippy::type_complexity)]
pub fn run_ghost_actions(
    decks: crate::decks::Decks,
    mut ghosts: Query<(&mut GridPos, &mut PendingActions, &mut Outbox), With<Ghost>>,
    ways: Query<(Option<&Ladder>, Option<&Stairs>), Without<Ghost>>,
    looks: Query<(&Appearance, Option<&DisplayName>)>,
    hidden: Query<(), With<rs13_world::Hidden>>,
) {
    use crate::actions::Action;
    for (mut pos, mut pending, mut outbox) in &mut ghosts {
        for action in pending.0.drain(..) {
            let (Action::UseOn(target) | Action::Examine(target)) = action else {
                continue;
            };
            if hidden.contains(target) {
                continue;
            }
            if matches!(action, Action::UseOn(_)) {
                let down = match ways.get(target) {
                    Ok((Some(ladder), _)) => Some(ladder.down),
                    Ok((_, Some(stairs))) => Some(stairs.top),
                    _ => None,
                };
                if let Some(down) = down {
                    let to: TilePos = other_deck(pos.0, down);
                    if decks.tile(to).is_some() {
                        pos.0 = to;
                    }
                    continue;
                }
            }
            match looks.get(target) {
                Ok((_, Some(name))) => {
                    outbox
                        .0
                        .push(LocMsg::new("examine-person").arg("name", name.0.clone()));
                }
                Ok((look, None)) => {
                    outbox
                        .0
                        .push(LocMsg::new("examine").arg("item", look.sprite.clone()));
                }
                Err(_) => {}
            }
        }
    }
}

/// What a ghost says only the dead hear, wherever they are (Luna `say_dead`).
pub fn ghost_speech(mut ghosts: Query<(&DisplayName, &mut PendingSpeech, &mut Outbox), With<Ghost>>) {
    let mut said: Vec<LocMsg> = Vec::new();
    for (name, mut pending, _) in &mut ghosts {
        for raw in pending.0.drain(..) {
            let Some(text) = sanitize_speech(&raw) else {
                continue;
            };
            said.push(
                LocMsg::new("ghost-say")
                    .arg("name", name.0.clone())
                    .arg("verb", speech_verb(&text))
                    .arg("text", text),
            );
        }
    }
    for (_, _, mut outbox) in &mut ghosts {
        outbox.0.extend(said.iter().cloned());
    }
}

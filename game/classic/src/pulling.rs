//! Pulling things and people along. Spec: `docs/reference/pulling.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Dense, Dir, GridPos, TilePos};

use crate::chairs::Chair;
use crate::closets::Closet;
use crate::generators::Pacman;
use crate::hands::Item;
use crate::machines::{PipeMember, Portable};
use crate::movement::{Mob, resolve_step};

/// On a mob: what it is pulling (Luna `mob.pulling`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pulling(pub Entity);

/// Steps mobs took this tick, and the tile each left, for what they pull to follow.
#[derive(Resource, Debug, Default)]
pub struct Stepped(pub Vec<(Entity, TilePos)>);

/// What can be pulled: anything on the map that isn't anchored down (Luna `anchored`).
#[derive(SystemParam)]
pub struct Pullable<'w, 's> {
    things: Query<
        'w,
        's,
        (
            Has<Item>,
            Has<Mob>,
            Has<Closet>,
            Has<Chair>,
            Option<&'static PipeMember>,
            Has<Portable>,
            Option<&'static Pacman>,
            Option<&'static crate::filing::FilingCabinet>,
            Has<crate::heaters::SpaceHeater>,
            Option<&'static crate::security::Barrier>,
            Option<&'static crate::nuke::NuclearBomb>,
            Option<&'static crate::anomalies::Artifact>,
        ),
        With<GridPos>,
    >,
}

impl Pullable<'_, '_> {
    /// Items, people, closets and crates, chairs, space heaters, an unlocked barrier, a canister not wrenched to
    /// a port, a generator or filing cabinet not bolted down, and an artifact no machine is working on.
    pub fn check(&self, target: Entity) -> bool {
        let Ok((
            item,
            mob,
            closet,
            chair,
            member,
            canister,
            generator,
            cabinet,
            heater,
            barrier,
            bomb,
            artifact,
        )) = self.things.get(target)
        else {
            return false;
        };
        item || mob
            || closet
            || chair
            || (canister && !member.is_some_and(|m| m.open))
            || generator.is_some_and(|g| !g.anchored)
            || cabinet.is_some_and(|c| !c.anchored)
            || heater
            || barrier.is_some_and(|b| !b.locked)
            || bomb.is_some_and(|b| !b.anchored)
            || artifact.is_some_and(|a| !a.busy)
    }
}

/// Whether `a` and `b` are on the same deck within a tile of each other.
fn near(a: TilePos, b: TilePos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

/// Pull requests made this tick: (mob, target).
#[derive(Resource, Debug, Default)]
pub struct PendingPulls(pub Vec<(Entity, Entity)>);

/// Start the pulls asked for this tick (Luna's Pull verb, Ctrl-click): the target must be
/// within a tile and not anchored. Silent, like Luna. Pulling something new lets go of the
/// old.
pub fn start_pulls(
    mut pending: ResMut<PendingPulls>,
    positions: Query<&GridPos>,
    pullable: Pullable,
    mut commands: Commands,
) {
    for (mob, target) in pending.0.drain(..) {
        let (Ok(mob_pos), Ok(target_pos)) = (positions.get(mob), positions.get(target)) else {
            continue;
        };
        if target != mob && near(mob_pos.0, target_pos.0) && pullable.check(target) {
            commands.entity(mob).insert(Pulling(target));
        }
    }
}

/// After the mobs move: whatever each one that stepped pulls follows into the tile it left
/// (Luna `carbon/Move`). A person is moved straight there; a thing takes one step toward
/// it, if nothing blocks it. If the pulled thing was already out of reach before the step,
/// the pull is let go instead. Something next to the puller, not diagonally, stays put.
pub fn follow_pullers(
    map: crate::decks::Decks,
    mut stepped: ResMut<Stepped>,
    pullers: Query<&Pulling, With<Mob>>,
    mut positions: Query<(Entity, &mut GridPos, Has<Dense>, Has<Mob>), Without<crate::firedoors::Firedoor>>,
    firedoors: Query<(&GridPos, &crate::firedoors::Firedoor), Without<Mob>>,
    panes: crate::windoors::EdgeBarriers<'_>,
    mut commands: Commands,
) {
    let mut sealed = crate::firedoors::closed_edges(firedoors.iter());
    sealed.extend(panes.iter());
    for (puller, left) in stepped.0.drain(..) {
        let Ok(pulling) = pullers.get(puller) else {
            continue;
        };
        let place = |e: Entity| positions.get(e).ok().map(|(_, p, ..)| p.0);
        let (Some(now_at), Some(pulled_at)) = (place(puller), place(pulling.0)) else {
            continue;
        };
        if !near(left, pulled_at) {
            commands.entity(puller).remove::<Pulling>();
            continue;
        }
        let diagonal = pulled_at.x != now_at.x && pulled_at.y != now_at.y;
        if near(now_at, pulled_at) && !diagonal {
            continue;
        }
        // Everything dense where it stands now, but the pulled thing itself.
        let occupied: std::collections::HashSet<TilePos> = positions
            .iter()
            .filter(|(e, _, dense, _)| *dense && *e != pulling.0)
            .map(|(_, p, ..)| p.0)
            .collect();
        let Ok((_, mut pos, _, is_mob)) = positions.get_mut(pulling.0) else {
            continue;
        };
        if is_mob {
            pos.0 = left;
        } else if let Some(dir) = Dir::from_offset((left.x - pos.0.x).signum(), (left.y - pos.0.y).signum())
            && let Some(to) = resolve_step(map.at(pos.0.z), &occupied, &sealed, pos.0, dir)
        {
            pos.0 = to;
        }
    }
}

/// Let go of what can't be pulled any more: picked up, gone, or anchored.
pub fn drop_lost_pulls(pullers: Query<(Entity, &Pulling)>, pullable: Pullable, mut commands: Commands) {
    for (puller, pulling) in &pullers {
        if !pullable.check(pulling.0) {
            commands.entity(puller).remove::<Pulling>();
        }
    }
}

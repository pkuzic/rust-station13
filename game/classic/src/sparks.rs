//! Sparks (Luna `spark_spread`): a few sparks fly from where something sparked, each hopping
//! one to three tiles in a straight line, half a second a hop, and touching the air at
//! 1 000 K wherever it lands, so a shower of sparks can light plasma next door. Each is
//! gone two seconds after its last hop. Spec: `docs/reference/igniters.md` (Sparks).

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dir, GridPos, MapLayer, NetId, TilePos};

use crate::atmos::StationAtmos;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

/// The sprite of a flying spark.
pub const SPARKS: &str = "sparks";
/// How hot a spark is where it lands, K (Luna `hotspot_expose(1000, 100)`).
pub const SPARK_HEAT: f64 = 1000.0;
/// Time between hops, ds (Luna `sleep(5)`).
pub const HOP_DS: f32 = 5.0;
/// How long a spark lingers after its last hop, ds (Luna `spawn(20)`).
pub const LINGER_DS: f32 = 20.0;

/// A flying spark: which way it goes, how many hops it has left, and when it next moves
/// (or, with no hops left, goes).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Spark {
    pub dir: Dir,
    pub hops: u8,
    pub next: f32,
}

/// Sparks asked for this tick: where, and how many.
#[derive(Resource, Debug, Default)]
pub struct SparkShowers(pub Vec<(TilePos, u8)>);

const CARDINALS: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];

/// Throw the sparks asked for, move the flying ones and take away the spent ones. Each
/// spark touches the air where it appears, at every hop, and where it goes out.
pub fn run_sparks(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut showers: ResMut<SparkShowers>,
    mut rng: ResMut<GameRng>,
    mut ids: ResMut<crate::objects::NetIds>,
    mut sounds: ResMut<SoundEvents>,
    atmos: Option<ResMut<StationAtmos>>,
    maps: Query<&MapLayer>,
    mut sparks: Query<(Entity, &mut Spark, &mut GridPos)>,
) {
    let now = time.deciseconds();
    let mut touched: Vec<TilePos> = Vec::new();
    for (at, n) in std::mem::take(&mut showers.0) {
        sounds.play(crate::sound::SPARKS, at, 100.0);
        for _ in 0..n {
            let dir = CARDINALS[rng.range(0, 3) as usize];
            let hops = rng.range(1, 3) as u8;
            let id: NetId = ids.alloc();
            commands.spawn((
                GridPos(at),
                id,
                Appearance::new(SPARKS, ""),
                Spark {
                    dir,
                    hops,
                    next: now + HOP_DS,
                },
            ));
            touched.push(at);
        }
    }
    for (entity, mut spark, mut pos) in &mut sparks {
        if now < spark.next {
            continue;
        }
        if spark.hops == 0 {
            touched.push(pos.0);
            commands.entity(entity).despawn();
            continue;
        }
        spark.hops -= 1;
        let to = pos.0.step(spark.dir);
        let blocked = maps.iter().any(|m| m.0.z == to.z && m.0.is_solid(to));
        if !blocked {
            pos.0 = to;
            touched.push(to);
        }
        spark.next = now + if spark.hops == 0 { LINGER_DS } else { HOP_DS };
    }
    if let Some(mut atmos) = atmos {
        for at in touched {
            atmos.0.expose(at, SPARK_HEAT);
        }
    }
}

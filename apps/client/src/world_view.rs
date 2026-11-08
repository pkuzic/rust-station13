//! Putting the world image on the screen (`scaling.rs` draws it): through
//! `world_view.wesl`, which bends the picture where the air is hot (`heat.rs`) and shows
//! how the player is (`docs/reference/hud.md`, Screen effects):
//!
//! - short of air, the edges of the view darken; badly hurt, they go dark red;
//! - when hurt, the view flashes red at the edges;
//! - unconscious, it blurs and darkens; dead, it turns grey and dark;
//! - cold, it turns a little blue; overheating, a little warm;
//! - blind, it is black (`genetics.md`), at once;
//! - blurred eyes blur it; nearsighted, all but the middle is dim and blurred (`eyes.md`).
//!
//! Each eases in and out over a fraction of a second rather than snapping.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use rs13_network::protocol::StatusUpdate;
use rs13_world::{EnvAlert, MobStat};

use crate::heat::HeatMap;
use crate::scaling::SCREEN_PX;
use crate::{OwnStatus, TILE_PX, WorldCamera};

/// How fast the effects ease toward where they are heading, per second.
const EASE_RATE: f32 = 4.0;
/// How fast the white-out of being dazzled by a flash fades, per second.
const DAZZLE_FADE: f32 = 0.8;
/// How fast the red flash of being hurt fades, per second.
const FLASH_FADE: f32 = 4.0;
/// How dark the edges get short of air, from 0 to 1.
const BREATHLESS: f32 = 0.6;
/// Health below which the edges start to go dark red, and how dark at its worst.
const HURT_FROM: f32 = 70.0;
const HURT_MOST: f32 = 0.85;
/// Body temperatures (K) past which the view starts to turn blue or warm, and how many
/// degrees further it is at its strongest.
const COLD_FROM: f32 = 300.0;
const COLD_SPAN: f32 = 25.0;
const HOT_FROM: f32 = 320.0;
const HOT_SPAN: f32 = 40.0;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WorldViewMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub world: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub heat: Handle<Image>,
    /// Camera centre x and y (world px), seconds, world px across the square.
    #[uniform(4)]
    pub view: Vec4,
    /// Map width and height (tiles), sprite px per tile, blind (0 or 1).
    #[uniform(5)]
    pub map: Vec4,
    /// Dark edges (short of air), red edges (hurt), red flash, unconscious: each 0 to 1.
    #[uniform(6)]
    pub effects: Vec4,
    /// Dead, cold, overheating, dazzled: each 0 to 1.
    #[uniform(7)]
    pub more_effects: Vec4,
    /// Blurred, nearsighted: each 0 to 1.
    #[uniform(8)]
    pub eyes: Vec4,
}

/// Where each screen effect is heading for the player's state, each from 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScreenEffects {
    pub breathless: f32,
    pub hurt: f32,
    pub flash: f32,
    pub unconscious: f32,
    pub dead: f32,
    pub cold: f32,
    pub hot: f32,
    pub dazzled: f32,
    pub blurry: f32,
    pub nearsighted: f32,
}

impl ScreenEffects {
    /// What the player's state calls for. The flash is not here: it starts when health
    /// drops ([`ScreenEffects::step`]).
    pub fn target(status: &StatusUpdate) -> Self {
        let temperature = f32::from(status.body_temperature);
        Self {
            breathless: if status.alerts.contains(&EnvAlert::LowOxygen) {
                BREATHLESS
            } else {
                0.0
            },
            hurt: ((HURT_FROM - status.health as f32) / HURT_FROM).clamp(0.0, 1.0) * HURT_MOST,
            flash: 0.0,
            unconscious: f32::from(u8::from(status.stat == MobStat::Unconscious)),
            dead: f32::from(u8::from(status.stat == MobStat::Dead)),
            cold: ((COLD_FROM - temperature) / COLD_SPAN).clamp(0.0, 1.0),
            hot: ((temperature - HOT_FROM) / HOT_SPAN).clamp(0.0, 1.0),
            dazzled: 0.0,
            blurry: f32::from(u8::from(status.eyes.blurry)),
            nearsighted: f32::from(u8::from(status.eyes.nearsighted)),
        }
    }

    /// `dt` seconds on: ease toward `target`, fade the flash, and flash again if `hit`.
    pub fn step(&mut self, target: &Self, dt: f32, hit: bool, dazzled: bool) {
        self.dazzled = if dazzled {
            1.0
        } else {
            self.dazzled * (-DAZZLE_FADE * dt).exp()
        };
        let k = 1.0 - (-EASE_RATE * dt).exp();
        let ease = |now: &mut f32, to: f32| *now += (to - *now) * k;
        ease(&mut self.breathless, target.breathless);
        ease(&mut self.hurt, target.hurt);
        ease(&mut self.unconscious, target.unconscious);
        ease(&mut self.dead, target.dead);
        ease(&mut self.cold, target.cold);
        ease(&mut self.hot, target.hot);
        ease(&mut self.blurry, target.blurry);
        ease(&mut self.nearsighted, target.nearsighted);
        self.flash = if hit {
            1.0
        } else {
            self.flash * (-FLASH_FADE * dt).exp()
        };
    }
}

impl WorldViewMaterial {
    pub fn new(world: Handle<Image>, heat: Handle<Image>) -> Self {
        Self {
            world,
            heat,
            view: Vec4::new(0.0, 0.0, 0.0, SCREEN_PX),
            map: Vec4::new(1.0, 1.0, TILE_PX, 0.0),
            effects: Vec4::ZERO,
            more_effects: Vec4::ZERO,
            eyes: Vec4::ZERO,
        }
    }
}

impl UiMaterial for WorldViewMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("world_view.wesl")).with_source("embedded"))
    }
}

pub struct WorldViewPlugin;

impl Plugin for WorldViewPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "world_view.wesl");
        app.add_plugins(UiMaterialPlugin::<WorldViewMaterial>::default())
            .init_resource::<Effects>()
            .add_systems(PostUpdate, update_view);
    }
}

/// The screen effects now, and the health they were last stepped with.
#[derive(Resource, Default)]
struct Effects {
    now: ScreenEffects,
    health: Option<i32>,
    dazzle: Option<u8>,
}

/// Tell the shader where the camera is, what time it is, how big the map is and how the
/// player is.
fn update_view(
    time: Res<Time>,
    status: Res<OwnStatus>,
    mut effects: ResMut<Effects>,
    heat: Option<Res<HeatMap>>,
    camera: Query<&Transform, With<WorldCamera>>,
    nodes: Query<&MaterialNode<WorldViewMaterial>>,
    mut materials: ResMut<Assets<WorldViewMaterial>>,
) {
    let Ok(camera) = camera.single() else { return };
    let size = heat.map_or((1, 1, 0), |h| h.size);
    let target = status.0.as_ref().map(ScreenEffects::target).unwrap_or_default();
    let health = status.0.as_ref().map(|s| s.health);
    let hit = matches!((effects.health, health), (Some(was), Some(now)) if now < was);
    effects.health = health;
    let dazzle = status.0.as_ref().map(|s| s.dazzled);
    let dazzled = matches!((effects.dazzle, dazzle), (Some(was), Some(now)) if now != was);
    effects.dazzle = dazzle;
    effects.now.step(&target, time.delta_secs(), hit, dazzled);
    let fx = effects.now;
    let blind = f32::from(u8::from(
        status
            .0
            .as_ref()
            .is_some_and(|s| s.vision == rs13_world::Vision::Blind),
    ));
    for node in &nodes {
        if let Some(mut material) = materials.get_mut(&node.0) {
            material.view = Vec4::new(
                camera.translation.x,
                camera.translation.y,
                time.elapsed_secs(),
                SCREEN_PX,
            );
            material.map = Vec4::new(size.0 as f32, size.1 as f32, TILE_PX, blind);
            material.effects = Vec4::new(fx.breathless, fx.hurt, fx.flash, fx.unconscious);
            material.more_effects = Vec4::new(fx.dead, fx.cold, fx.hot, fx.dazzled);
            material.eyes = Vec4::new(fx.blurry, fx.nearsighted, 0.0, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use rs13_world::Intent;

    use super::*;

    fn status(health: i32, stat: MobStat, alerts: Vec<EnvAlert>, kelvin: u16) -> StatusUpdate {
        StatusUpdate {
            dazzled: 0,
            health,
            stat,
            intent: Intent::Help,
            pressure_dkpa: 1013,
            alerts,
            internals: false,
            body_temperature: kelvin,
            pulling: false,
            vision: rs13_world::Vision::Normal,
            eyes: rs13_world::EyeTrouble::default(),
        }
    }

    #[test]
    fn a_healthy_player_sees_the_plain_view() {
        let fx = ScreenEffects::target(&status(100, MobStat::Conscious, vec![], 310));
        assert_eq!(fx, ScreenEffects::default());
    }

    #[test]
    fn the_players_state_shows_on_the_view() {
        let fx = ScreenEffects::target(&status(35, MobStat::Unconscious, vec![EnvAlert::LowOxygen], 275));
        assert_eq!(fx.breathless, BREATHLESS);
        assert!((fx.hurt - 0.5 * HURT_MOST).abs() < 1e-6, "half way from 70 to 0");
        assert_eq!(fx.unconscious, 1.0);
        assert_eq!(fx.dead, 0.0);
        assert_eq!(fx.cold, 1.0, "25 K below 300");
        assert_eq!(fx.hot, 0.0);
        let dead = ScreenEffects::target(&status(-150, MobStat::Dead, vec![], 380));
        assert_eq!((dead.dead, dead.hurt, dead.hot), (1.0, HURT_MOST, 1.0));
    }

    #[test]
    fn effects_ease_in_and_a_hit_flashes_then_fades() {
        let target = ScreenEffects {
            unconscious: 1.0,
            ..Default::default()
        };
        let mut fx = ScreenEffects::default();
        fx.step(&target, 0.1, true, false);
        assert!(fx.unconscious > 0.2 && fx.unconscious < 0.5, "{}", fx.unconscious);
        assert_eq!(fx.flash, 1.0);
        for _ in 0..20 {
            fx.step(&target, 0.1, false, false);
        }
        assert!(fx.unconscious > 0.99);
        assert!(fx.flash < 0.01);
    }
}

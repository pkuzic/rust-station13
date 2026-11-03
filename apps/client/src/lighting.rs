//! Soft coloured lighting over the map (`docs/reference/lighting.md`).
//!
//! The light on each tile comes from the light sources the server replicates, worked out
//! per tile by `rs13_simulation::lighting` as Luna does, plus the faint light of glowing
//! screens (`glow.rs`). It is drawn as a texture with one texel per tile, blended smoothly
//! between the tiles the player sees and multiplied over everything on the map. F4 turns
//! it off.

use std::hash::{Hash, Hasher};

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::image::ImageSampler;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d, RenderPipelineDescriptor,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPipeline, Material2dPlugin};
use rs13_simulation::lighting::{Emitter, MapDecks};
use rs13_world::{GridPos, Hidden, LightSource, MapLayer, Opaque, TileMap, TilePos};

use crate::shadows::Sight;
use crate::{ChatInput, OwnPlayer, TILE_PX};

/// Above every sprite on the map.
const LIGHT_Z: f32 = 20.0;
/// The darkest a tile gets: dim but readable, so a dark room still shows its shape (what
/// stands in it may be only a silhouette), slightly cool, as darkness reads.
const MIN_LIGHT: [f32; 3] = [0.22, 0.22, 0.27];
/// See `shown_light`.
const LIGHT_SCALE: f32 = 0.85;
const LIGHT_CURVE: f32 = 1.3;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lighting.wgsl");
        app.add_plugins(Material2dPlugin::<LightMaterial>::default())
            .insert_resource(LightingOn(true))
            .init_resource::<LightCache>()
            .add_systems(
                Update,
                (toggle_lighting, update_lighting, present_light)
                    .chain()
                    .in_set(DrawLight),
            );
    }
}

/// Working out and drawing the light, after what the player sees (`shadows.rs`).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DrawLight;

/// Whether lighting is drawn (F4).
#[derive(Resource)]
pub struct LightingOn(pub bool);

/// The darkness over the map.
#[derive(Component)]
struct LightOverlay {
    width: u32,
    height: u32,
    image: Handle<Image>,
}

/// Multiplies the light texture over what is drawn under it.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct LightMaterial {
    #[texture(0)]
    light: Handle<Image>,
}

impl Material2d for LightMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("lighting.wgsl")).with_source("embedded"))
    }

    // Drawn with the sprites, in order of depth, so it lands on top of them.
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let multiply = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(multiply);
            }
        }
        Ok(())
    }
}

fn toggle_lighting(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    became_ghost: Query<(), (Added<OwnPlayer>, With<rs13_world::Ghost>)>,
    became_person: Query<(), (Added<OwnPlayer>, Without<rs13_world::Ghost>)>,
    mut on: ResMut<LightingOn>,
) {
    // Ghosts see in the dark and through walls (`ghosts.md`). F4 brings the dark back. A new
    // character after that sees as anyone does.
    if !became_ghost.is_empty() {
        on.0 = false;
    }
    if !became_person.is_empty() {
        on.0 = true;
    }
    if !chat.open && keys.just_pressed(KeyCode::F4) {
        on.0 = !on.0;
    }
}

/// The last light worked out, kept so moving about only redoes what the player sees.
#[derive(Resource, Default)]
struct LightCache {
    digest: Option<u64>,
    width: u32,
    height: u32,
    z: i32,
    levels: Vec<[f32; 3]>,
    /// Per texel, top row first: the light to show, and whether the tile is in sight.
    target: Vec<([f32; 3], bool)>,
    /// The light shown now, easing towards `target` (and the fires).
    shown: Vec<[f32; 3]>,
    /// Fires on our deck, whose light flickers.
    fires: Vec<Emitter>,
    opaque: Vec<TilePos>,
}

/// How fast shown light eases to a change, per second: about a third of a second to settle.
const FADE_RATE: f32 = 10.0;

/// How light looks: a little dimmer than the game's level, falling away faster from its source, so
/// rooms have more shade. It's only how it's drawn: what counts as dark doesn't change.
fn shown_light(level: f32) -> f32 {
    level.powf(LIGHT_CURVE) * LIGHT_SCALE
}

/// The texture's bytes: light per texel (no darker than `MIN_LIGHT`), and in alpha whether
/// the tile gives light to the picture (`lighting.wgsl`).
fn texture_data(texels: impl Iterator<Item = ([f32; 3], bool)>) -> Vec<u8> {
    let mut data = Vec::new();
    for (light, in_sight) in texels {
        for (c, min) in light.into_iter().zip(MIN_LIGHT) {
            data.push((shown_light(c).max(min) * 255.0).round() as u8);
        }
        data.push(if in_sight { 255 } else { 0 });
    }
    data
}

/// How strongly a fire burns this moment, from 0.75 to 1: two quick waves, offset per tile
/// so fires don't flicker in step.
pub fn flicker(seconds: f32, pos: TilePos) -> f32 {
    let phase = (pos.x.wrapping_mul(73) ^ pos.y.wrapping_mul(151)) as f32 * 0.37;
    let wave = 0.5 * (seconds * 11.0 + phase).sin() + 0.5 * (seconds * 17.3 + 2.0 * phase).sin();
    0.875 + 0.125 * wave
}

/// Every frame: ease the shown light towards what it should be, with the fires' light
/// flickering on top, and upload it when it changed.
fn present_light(
    time: Res<Time>,
    on: Res<LightingOn>,
    mut cache: ResMut<LightCache>,
    maps: Query<&MapLayer>,
    overlays: Query<&LightOverlay>,
    mut images: ResMut<Assets<Image>>,
) {
    let (width, height) = (cache.width, cache.height);
    let n = (width * height) as usize;
    if !on.0 || cache.target.len() != n {
        return;
    }
    let Some(overlay) = overlays.iter().find(|o| o.width == width && o.height == height) else {
        return;
    };
    // The fires' light this frame.
    let mut fire = vec![[0.0_f32; 3]; n];
    if let Some(map) = maps.iter().find(|m| m.0.z == cache.z)
        && !cache.fires.is_empty()
    {
        let now = time.elapsed_secs();
        let lit: Vec<Emitter> = cache
            .fires
            .iter()
            .map(|e| Emitter {
                pos: e.pos,
                rgb: e.rgb.map(|c| c * flicker(now, e.pos)),
            })
            .collect();
        let field = MapDecks::new([&map.0], cache.opaque.iter().copied()).light(&lit);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let pos = TilePos::new(x, y, cache.z);
                if map
                    .0
                    .tile(pos)
                    .is_some_and(|t| t.id == rs13_simulation::lighting::SPACE)
                {
                    continue;
                }
                fire[((height as i32 - 1 - y) * width as i32 + x) as usize] = field.level(pos);
            }
        }
    }
    let desired: Vec<[f32; 3]> = cache
        .target
        .iter()
        .zip(&fire)
        .map(|((light, _), f)| std::array::from_fn(|c| (light[c] + f[c]).min(1.0)))
        .collect();
    let step = 1.0 - (-time.delta_secs() * FADE_RATE).exp();
    let mut changed = cache.shown.len() != n;
    if changed {
        cache.shown = desired;
    } else {
        for (shown, want) in cache.shown.iter_mut().zip(&desired) {
            for c in 0..3 {
                let gap = want[c] - shown[c];
                if gap.abs() > 1.0 / 512.0 {
                    shown[c] += gap * step;
                    changed = true;
                } else if gap != 0.0 {
                    shown[c] = want[c];
                    changed = true;
                }
            }
        }
    }
    if !changed {
        return;
    }
    let data = texture_data(
        cache
            .shown
            .iter()
            .zip(&cache.target)
            .map(|(light, (_, in_sight))| (*light, *in_sight)),
    );
    if let Some(mut image) = images.get_mut(&overlay.image) {
        image.data = Some(data);
    }
}

/// The light to draw on a wall or closed door: the light just outside its faces on the
/// side the player sees, the brightest of the open tiles beside it in sight, and only on
/// the faces that turn towards the player's eye (in tiles; `None` before our player
/// arrives). So light from a room behind a wall, or on a window beside a door, or falling
/// past its corner, never shows on the face we see. A room's corner, with no face to see,
/// takes the light at its corner instead. `None` if no such tile is in sight.
pub fn wall_light(
    at: TilePos,
    eye: Option<[f32; 2]>,
    light_at: impl Fn([f32; 2]) -> [f32; 3],
    blocks: impl Fn(TilePos) -> bool,
    seen: impl Fn(TilePos) -> bool,
) -> Option<[f32; 3]> {
    let brightest = |sides: &[(i32, i32)]| {
        sides
            .iter()
            .filter(|(dx, dy)| {
                let p = TilePos::new(at.x + dx, at.y + dy, at.z);
                // The face or corner on that side, and whether it turns towards the eye.
                let side = [at.x as f32 + 0.5 * *dx as f32, at.y as f32 + 0.5 * *dy as f32];
                let faces =
                    eye.is_none_or(|e| (e[0] - side[0]) * *dx as f32 + (e[1] - side[1]) * *dy as f32 > 0.0);
                faces && !blocks(p) && seen(p)
            })
            // Just past the wall's face, or its corner, into that tile.
            .map(|(dx, dy)| light_at([at.x as f32 + 0.55 * *dx as f32, at.y as f32 + 0.55 * *dy as f32]))
            .reduce(|a, b| std::array::from_fn(|c| a[c].max(b[c])))
    };
    // Its faces; only a room's corner, with no face to see, takes the light at its corner.
    brightest(&[(0, 1), (0, -1), (1, 0), (-1, 0)])
        .or_else(|| brightest(&[(1, 1), (1, -1), (-1, 1), (-1, -1)]))
}

/// Work out the light again when the map, a light or a door changed, and redraw it when
/// what the player sees changes.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    mut commands: Commands,
    on: Res<LightingOn>,
    mut cache: ResMut<LightCache>,
    sight: Res<Sight>,
    maps: Query<Ref<MapLayer>>,
    sources: Query<(&GridPos, &LightSource, Option<&rs13_world::Appearance>), Without<Hidden>>,
    (screens, glow): (
        Query<(&GridPos, &crate::sprites::Look), (Without<LightSource>, Without<Hidden>)>,
        Option<Res<crate::glow::GlowAtlas>>,
    ),
    doors: Query<&GridPos, With<Opaque>>,
    own: Query<(&GridPos, Option<&rs13_world::CameraView>), With<OwnPlayer>>,
    mut overlays: Query<(Entity, &LightOverlay, &mut Visibility)>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LightMaterial>>,
) {
    for (_, _, mut visibility) in &mut overlays {
        visibility.set_if_neq(if on.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    if !on.0 {
        return;
    }
    let decks: Vec<&TileMap> = maps.iter().map(|m| &m.into_inner().0).collect();
    let me = own.iter().next().map(|(p, view)| view.map_or(p.0, |v| v.0));
    // The deck we stand on, or the first one before our mob arrives.
    let Some(map) = me
        .and_then(|p| decks.iter().find(|m| m.z == p.z))
        .or_else(|| decks.iter().min_by_key(|m| m.z))
        .copied()
    else {
        return;
    };

    // Fires flicker, so their light is added afresh every frame (`present_light`).
    let (fires, steady): (Vec<_>, Vec<_>) = sources
        .iter()
        .map(|(pos, LightSource(rgb), look)| {
            let emitter = Emitter {
                pos: pos.0,
                rgb: rgb.map(f32::from),
            };
            (emitter, look.is_some_and(|a| a.sprite == "fire"))
        })
        .partition(|(_, fire)| *fire);
    let mut fires: Vec<Emitter> = fires
        .into_iter()
        .map(|(e, _)| e)
        .filter(|e| e.pos.z == map.z)
        .collect();
    fires.sort_by_key(|e| e.pos);
    let mut emitters: Vec<Emitter> = steady.into_iter().map(|(e, _)| e).collect();
    // Glowing screens cast a faint light of their own colour (`glow.rs`).
    if let Some(glow) = &glow {
        emitters.extend(screens.iter().filter_map(|(pos, look)| {
            Some(Emitter {
                pos: pos.0,
                rgb: glow.screen_light(&look.sprite, &look.state)?,
            })
        }));
    }
    emitters.sort_by_key(|e| e.pos);
    let mut opaque: Vec<TilePos> = doors.iter().map(|p| p.0).collect();
    opaque.sort();

    let mut digest = std::collections::hash_map::DefaultHasher::new();
    map.z.hash(&mut digest);
    for m in &maps {
        m.last_changed().hash(&mut digest);
    }
    for e in &emitters {
        e.pos.hash(&mut digest);
        e.rgb.map(f32::to_bits).hash(&mut digest);
    }
    opaque.hash(&mut digest);
    for e in &fires {
        e.pos.hash(&mut digest);
        e.rgb.map(f32::to_bits).hash(&mut digest);
    }
    let digest = digest.finish();
    let relit = cache.digest != Some(digest);
    if !relit && !sight.is_changed() && !overlays.is_empty() {
        return;
    }

    let (width, height) = (map.width, map.height);
    if relit || cache.z != map.z || cache.width != width || cache.height != height {
        let field = MapDecks::new(decks.iter().copied(), opaque.iter().copied()).light(&emitters);
        cache.levels = (0..height as i32)
            .flat_map(|y| (0..width as i32).map(move |x| (x, y)))
            .map(|(x, y)| field.level(TilePos::new(x, y, map.z)))
            .collect();
        cache.digest = Some(digest);
        cache.width = width;
        cache.height = height;
        cache.z = map.z;
    }

    let level = |x: i32, y: i32| {
        let inside = (0..width as i32).contains(&x) && (0..height as i32).contains(&y);
        if inside {
            cache.levels[(y as u32 * width + x as u32) as usize]
        } else {
            [0.0; 3]
        }
    };
    let blocks = |p: TilePos| map.is_opaque(p) || opaque.binary_search(&p).is_ok();
    let inside = |p: TilePos| (0..width as i32).contains(&p.x) && (0..height as i32).contains(&p.y);
    let seen = |p: TilePos| inside(p) && sight.sees(p);
    let eye = me.map(|p| [p.x as f32, p.y as f32]);
    let mut target = Vec::with_capacity((width * height) as usize);
    // Texture rows run from the top of the map down.
    for row in 0..height as i32 {
        let y = height as i32 - 1 - row;
        for x in 0..width as i32 {
            let tile = TilePos::new(x, y, map.z);
            // Walls and closed doors show the light on the side the player sees.
            target.push(if blocks(tile) {
                let at_point = |p: [f32; 2]| level(p[0].round() as i32, p[1].round() as i32);
                match wall_light(tile, eye, at_point, blocks, seen) {
                    Some(light) => (light, true),
                    None => (level(x, y), false),
                }
            } else {
                (level(x, y), seen(tile))
            });
        }
    }
    cache.target = target;
    cache.fires = fires;
    cache.opaque = opaque;

    let existing = overlays
        .iter()
        .any(|(_, o, _)| o.width == width && o.height == height);
    if existing {
        return;
    }
    let data = texture_data(cache.target.iter().map(|(light, in_sight)| (*light, *in_sight)));
    cache.shown.clear();
    for (entity, ..) in &overlays {
        commands.entity(entity).despawn();
    }
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let image = images.add(image);
    let size = Vec2::new(width as f32, height as f32) * TILE_PX;
    // Tile (x, y) is centred on (x, y) tiles; the texture's texels on the tile centres.
    let centre = (Vec2::new(width as f32, height as f32) - 1.0) * TILE_PX / 2.0;
    commands.spawn((
        LightOverlay {
            width,
            height,
            image: image.clone(),
        },
        Mesh2d(meshes.add(Rectangle::from_size(size))),
        MeshMaterial2d(materials.add(LightMaterial { light: image })),
        Transform::from_translation(centre.extend(LIGHT_Z)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_flickers_between_three_quarters_and_full() {
        let pos = TilePos::new(3, 4, 0);
        let samples: Vec<f32> = (0..200).map(|i| flicker(i as f32 * 0.05, pos)).collect();
        assert!(samples.iter().all(|f| (0.75..=1.0).contains(f)));
        let (lo, hi) = samples
            .iter()
            .fold((1.0_f32, 0.0_f32), |(lo, hi), f| (lo.min(*f), hi.max(*f)));
        assert!(hi - lo > 0.1, "it visibly flickers: {lo}..{hi}");
        assert_ne!(
            flicker(1.0, pos),
            flicker(1.0, TilePos::new(4, 4, 0)),
            "not in step"
        );
    }

    /// A 3 × 1 deck with an overlay, ready for `present_light`.
    fn presenting(target: [f32; 3], fires: Vec<Emitter>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).init_resource::<Assets<Image>>();
        let map = TileMap::from_ron(r#"(legend: { '.': (tile: (id: "floor")) }, rows: ["..."])"#, 0).unwrap();
        app.world_mut().spawn(MapLayer(map));
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut().spawn(LightOverlay {
            width: 3,
            height: 1,
            image,
        });
        app.insert_resource(LightingOn(true)).insert_resource(LightCache {
            width: 3,
            height: 1,
            target: vec![(target, true); 3],
            fires,
            ..default()
        });
        app.add_systems(Update, present_light);
        app
    }

    fn shown(app: &App) -> Vec<[f32; 3]> {
        app.world().resource::<LightCache>().shown.clone()
    }

    #[test]
    fn a_change_of_light_fades_in_rather_than_snapping() {
        let mut app = presenting([1.0; 3], vec![]);
        app.update();
        assert_eq!(shown(&app)[0], [1.0; 3]);
        // The light goes out: a frame later it is on its way down, not there yet.
        app.world_mut().resource_mut::<LightCache>().target = vec![([0.0; 3], true); 3];
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();
        let now = shown(&app)[0][0];
        assert!(now > 0.05 && now < 0.95, "fading: {now}");
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            app.update();
        }
        assert!(shown(&app)[0][0] < 0.01, "settled");
    }

    #[test]
    fn a_fire_lights_its_tile_and_its_light_wavers() {
        let fire = Emitter {
            pos: TilePos::new(1, 0, 0),
            rgb: [4.0, 2.0, 1.0],
        };
        let mut app = presenting([0.0; 3], vec![fire]);
        let mut reds = Vec::new();
        for _ in 0..12 {
            std::thread::sleep(std::time::Duration::from_millis(30));
            app.update();
            reds.push(shown(&app)[1][0]);
        }
        assert!(
            reds.iter().all(|r| *r > 0.2),
            "the fire lights its tile: {reds:?}"
        );
        let (lo, hi) = reds
            .iter()
            .fold((1.0_f32, 0.0_f32), |(lo, hi), r| (lo.min(*r), hi.max(*r)));
        assert!(hi - lo > 0.01, "and wavers: {reds:?}");
    }

    #[test]
    fn walls_show_the_light_on_the_side_you_see() {
        // A wall along x = 0: the room behind it (x < 0) is lit, ours (x > 0) is dark, and
        // only ours is in sight.
        let light_at = |p: [f32; 2]| if p[0] < 0.0 { [1.0; 3] } else { [0.1; 3] };
        let blocks = |p: TilePos| p.x == 0;
        let ours = |p: TilePos| p.x > 0;
        let wall = TilePos::new(0, 0, 0);
        assert_eq!(
            wall_light(wall, None, light_at, blocks, ours),
            Some([0.1; 3]),
            "not the light behind it"
        );
        assert_eq!(
            wall_light(wall, None, light_at, blocks, |_| false),
            None,
            "a wall out of sight"
        );
    }

    #[test]
    fn a_door_shows_the_light_on_the_face_you_see_not_a_lit_window_beside_it() {
        // A door at (0, 2) in a wall row, a lit window east of it with its light falling on
        // the floor below it, a dark room south where we stand at (0, 0).
        let blocks = |p: TilePos| p.y == 2 && p.x != 1;
        let seen = |p: TilePos| p.y <= 2;
        let light_at = |p: [f32; 2]| {
            if p[0] > 0.5 { [1.0; 3] } else { [0.05; 3] }
        };
        let door = TilePos::new(0, 2, 0);
        assert_eq!(
            wall_light(door, Some([0.0, 0.0]), light_at, blocks, seen),
            Some([0.05; 3])
        );
        // From the window's side the lit face is the one seen.
        assert_eq!(
            wall_light(door, Some([3.0, 2.0]), light_at, blocks, seen),
            Some([1.0; 3])
        );
    }

    #[test]
    fn a_room_corner_takes_the_light_of_the_floor_at_its_corner() {
        // Walls along x = 0 and y = 0; our room is x > 0, y > 0; the rest is lit.
        let blocks = |p: TilePos| (p.x == 0 && p.y >= 0) || (p.y == 0 && p.x >= 0);
        let ours = |p: TilePos| p.x > 0 && p.y > 0;
        let light_at = |p: [f32; 2]| {
            if p[0] > 0.0 && p[1] > 0.0 {
                [0.1; 3]
            } else {
                [1.0; 3]
            }
        };
        assert_eq!(
            wall_light(TilePos::new(0, 0, 0), None, light_at, blocks, ours),
            Some([0.1; 3])
        );
    }
}

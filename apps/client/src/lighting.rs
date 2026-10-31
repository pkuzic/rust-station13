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
/// The darkest a tile gets, so that people can still make out a dark room.
const MIN_LIGHT: f32 = 0.04;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lighting.wgsl");
        app.add_plugins(Material2dPlugin::<LightMaterial>::default())
            .insert_resource(LightingOn(true))
            .init_resource::<LightCache>()
            .add_systems(
                Update,
                (toggle_lighting, update_lighting).chain().in_set(DrawLight),
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

fn toggle_lighting(keys: Res<ButtonInput<KeyCode>>, chat: Res<ChatInput>, mut on: ResMut<LightingOn>) {
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
    sources: Query<(&GridPos, &LightSource), Without<Hidden>>,
    (screens, glow): (
        Query<(&GridPos, &crate::sprites::Look), (Without<LightSource>, Without<Hidden>)>,
        Option<Res<crate::glow::GlowAtlas>>,
    ),
    doors: Query<&GridPos, With<Opaque>>,
    own: Query<&GridPos, With<OwnPlayer>>,
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
    let me = own.iter().next().map(|p| p.0);
    // The deck we stand on, or the first one before our mob arrives.
    let Some(map) = me
        .and_then(|p| decks.iter().find(|m| m.z == p.z))
        .or_else(|| decks.iter().min_by_key(|m| m.z))
        .copied()
    else {
        return;
    };

    let mut emitters: Vec<Emitter> = sources
        .iter()
        .map(|(pos, LightSource(rgb))| Emitter {
            pos: pos.0,
            rgb: rgb.map(f32::from),
        })
        .collect();
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
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    // Texture rows run from the top of the map down.
    for row in 0..height as i32 {
        let y = height as i32 - 1 - row;
        for x in 0..width as i32 {
            let tile = TilePos::new(x, y, map.z);
            // Walls and closed doors show the light on the side the player sees.
            let (light, in_sight) = if blocks(tile) {
                let at_point = |p: [f32; 2]| level(p[0].round() as i32, p[1].round() as i32);
                match wall_light(tile, eye, at_point, blocks, seen) {
                    Some(light) => (light, true),
                    None => (level(x, y), false),
                }
            } else {
                (level(x, y), seen(tile))
            };
            for c in light.map(|c| c.max(MIN_LIGHT)) {
                data.push((c * 255.0).round() as u8);
            }
            // Alpha: whether this tile gives light to the picture (`lighting.wgsl`).
            data.push(if in_sight { 255 } else { 0 });
        }
    }

    let existing = overlays
        .iter()
        .find(|(_, o, _)| o.width == width && o.height == height)
        .map(|(_, o, _)| o.image.clone());
    if let Some(handle) = existing
        && let Some(mut image) = images.get_mut(&handle)
    {
        image.data = Some(data);
        return;
    }
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

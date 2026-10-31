//! Soft coloured lighting over the map (`docs/reference/lighting.md`).
//!
//! The light on each tile comes from the light sources the server replicates, worked out
//! by `rs13_simulation::lighting`. It is drawn as a texture with one texel per tile,
//! blended smoothly between tiles and multiplied over everything on the map. F4 turns it
//! off.

use std::collections::HashSet;
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
use rs13_world::{
    Appearance, GridPos, Hidden, LightSource, MapLayer, NetId, Opaque, Slot, TileMap, TilePos, WornView,
};

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
            .init_resource::<Sight>()
            .add_systems(Update, (toggle_lighting, update_lighting).chain());
    }
}

/// Whether lighting is drawn (F4).
#[derive(Resource)]
struct LightingOn(bool);

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

/// What the player can see right now (Luna's `view`): tiles out of sight are drawn black,
/// or dim with meson scanners on, and nothing on them is drawn.
#[derive(Resource, Default)]
pub struct Sight {
    /// Tiles of our deck the player can't see.
    pub unseen: HashSet<TilePos>,
    /// Meson scanners on: the floor plan shows through walls (Luna `SEE_TURFS`).
    pub meson: bool,
}

/// How far the player sees: Luna's 15 × 15 view.
const VIEW_RANGE: i32 = 7;
/// Brightness of a tile seen only through meson scanners.
const MESON_LIGHT: f32 = 0.22;

/// The last light worked out, kept so moving about only redoes what the player sees.
#[derive(Resource, Default)]
struct LightCache {
    digest: Option<u64>,
    sight: Option<u64>,
    width: u32,
    height: u32,
    z: i32,
    levels: Vec<[f32; 3]>,
}

/// Work out the light again when the map, a light or a door changed, and what the player
/// sees when they move; draw both.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    mut commands: Commands,
    on: Res<LightingOn>,
    mut cache: ResMut<LightCache>,
    mut sight: ResMut<Sight>,
    maps: Query<Ref<MapLayer>>,
    sources: Query<(&GridPos, &LightSource), Without<Hidden>>,
    doors: Query<&GridPos, With<Opaque>>,
    own: Query<(&GridPos, Option<&WornView>), With<OwnPlayer>>,
    items: Query<(&NetId, &Appearance)>,
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
        // The debug view shows everything.
        if !sight.unseen.is_empty() {
            sight.unseen.clear();
        }
        cache.sight = None;
        return;
    }
    let decks: Vec<&TileMap> = maps.iter().map(|m| &m.into_inner().0).collect();
    let me = own.iter().next();
    // The deck we stand on, or the first one before our mob arrives.
    let Some(map) = me
        .and_then(|(p, _)| decks.iter().find(|m| m.z == p.0.z))
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

    // Wearing meson scanners?
    let meson = me
        .and_then(|(_, worn)| worn?.get(Slot::Glasses))
        .and_then(|id| items.iter().find(|(n, _)| **n == id))
        .is_some_and(|(_, a)| a.sprite == "meson-glasses");
    let mut view = std::collections::hash_map::DefaultHasher::new();
    digest.hash(&mut view);
    me.map(|(p, _)| p.0).hash(&mut view);
    meson.hash(&mut view);
    let view = view.finish();
    if cache.digest == Some(digest) && cache.sight == Some(view) && !overlays.is_empty() {
        return;
    }

    let (width, height) = (map.width, map.height);
    if cache.digest != Some(digest) || cache.z != map.z || cache.width != width || cache.height != height {
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
    cache.sight = Some(view);

    // Line of sight from our tile, within the view (as speech carries).
    let mut unseen = HashSet::new();
    if let Some((pos, _)) = me {
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let tile = TilePos::new(x, y, map.z);
                if !rs13_classic::speech::can_see(map, &opaque, pos.0, tile, VIEW_RANGE) {
                    unseen.insert(tile);
                }
            }
        }
    }

    let mut data = Vec::with_capacity((width * height * 4) as usize);
    // Texture rows run from the top of the map down.
    for row in 0..height as i32 {
        let y = height as i32 - 1 - row;
        for x in 0..width as i32 {
            let tile = TilePos::new(x, y, map.z);
            let level = if !unseen.contains(&tile) {
                cache.levels[(y as u32 * width + x as u32) as usize].map(|c| c.max(MIN_LIGHT))
            } else if meson {
                [MESON_LIGHT; 3]
            } else {
                [0.0; 3]
            };
            for c in level {
                data.push((c * 255.0).round() as u8);
            }
            data.push(255);
        }
    }
    sight.unseen = unseen;
    sight.meson = meson;

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

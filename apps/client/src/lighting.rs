//! Soft coloured lighting over the map (`docs/reference/lighting.md`).
//!
//! The light on each tile comes from the light sources the server replicates, worked out
//! by `rs13_simulation::lighting`. It is drawn as a texture with one texel per tile,
//! blended smoothly between tiles and multiplied over everything on the map. F4 turns it
//! off.

use std::collections::HashMap;
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
use rs13_simulation::lighting::{Emitter, LightField, LightWorld};
use rs13_world::{GridPos, Hidden, LightSource, MapLayer, Opaque, TileMap, TilePos};

use crate::{ChatInput, OwnPlayer, TILE_PX};

/// Above every sprite on the map.
const LIGHT_Z: f32 = 20.0;
/// The darkest a tile gets, so that people can still make out a dark room.
const MIN_LIGHT: f32 = 0.04;
/// A tile with no floor, which light passes through to the deck below. No map has one
/// yet.
pub const OPEN_SPACE: &str = "open-space";

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lighting.wgsl");
        app.add_plugins(Material2dPlugin::<LightMaterial>::default())
            .insert_resource(LightingOn(true))
            .init_resource::<LightInputs>()
            .add_systems(Update, (toggle_lighting, update_lighting).chain());
    }
}

/// Whether lighting is drawn (F4).
#[derive(Resource)]
struct LightingOn(bool);

/// A digest of what the light was last worked out from, to skip unchanged frames.
#[derive(Resource, Default)]
struct LightInputs(Option<u64>);

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

/// The decks the client knows, and the closed doors on them.
struct ClientDecks<'a> {
    decks: HashMap<i32, &'a TileMap>,
    opaque: &'a [TilePos],
}

impl LightWorld for ClientDecks<'_> {
    fn is_opaque(&self, pos: TilePos) -> bool {
        self.decks.get(&pos.z).is_some_and(|m| m.is_opaque(pos)) || self.opaque.contains(&pos)
    }

    fn is_open_below(&self, pos: TilePos) -> bool {
        self.decks
            .get(&pos.z)
            .and_then(|m| m.tile(pos))
            .is_some_and(|t| t.id == OPEN_SPACE)
    }
}

fn toggle_lighting(keys: Res<ButtonInput<KeyCode>>, chat: Res<ChatInput>, mut on: ResMut<LightingOn>) {
    if !chat.open && keys.just_pressed(KeyCode::F4) {
        on.0 = !on.0;
    }
}

/// Work out the light again when the map, a light or a door changed, and draw it.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    mut commands: Commands,
    on: Res<LightingOn>,
    mut last: ResMut<LightInputs>,
    maps: Query<Ref<MapLayer>>,
    sources: Query<(&GridPos, &LightSource), Without<Hidden>>,
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
    let decks: HashMap<i32, &TileMap> = maps.iter().map(|m| (m.0.z, &m.into_inner().0)).collect();
    // The deck we stand on, or the first one before our mob arrives.
    let Some(map) = own
        .iter()
        .next()
        .and_then(|p| decks.get(&p.0.z))
        .or_else(|| decks.values().min_by_key(|m| m.z))
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
    if last.0 == Some(digest) && !overlays.is_empty() {
        return;
    }
    last.0 = Some(digest);

    let world = ClientDecks {
        decks: decks.clone(),
        opaque: &opaque,
    };
    let mut field = LightField::compute(&world, &emitters);
    // Space has no lighting of its own: it is always lit (Luna areas without `ul_Lighting`).
    for deck in decks.values() {
        for y in 0..deck.height as i32 {
            for x in 0..deck.width as i32 {
                let pos = TilePos::new(x, y, deck.z);
                if deck.tile(pos).is_some_and(|t| t.id == "space") {
                    field.light_fully(pos);
                }
            }
        }
    }

    let (width, height) = (map.width, map.height);
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    // Texture rows run from the top of the map down.
    for row in 0..height as i32 {
        let y = height as i32 - 1 - row;
        for x in 0..width as i32 {
            let level = field.level(TilePos::new(x, y, map.z));
            for c in level {
                data.push((c.max(MIN_LIGHT) * 255.0).round() as u8);
            }
            data.push(255);
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

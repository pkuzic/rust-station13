//! Soft coloured lighting over the map (`docs/reference/lighting.md`).
//!
//! The light comes from the light sources the server replicates, cast geometrically past
//! walls and closed doors (`light_map.rs`) with the falloff `rs13_simulation::lighting`
//! uses per tile. It is drawn as a texture, sixteen texels to a tile, sampled smoothly and
//! multiplied over everything on the map. F4 turns it off.

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
use rs13_simulation::lighting::SPACE;
use rs13_world::{
    Appearance, GridPos, Hidden, LightSource, MapLayer, NetId, Opaque, Slot, TileMap, TilePos, WornView,
};

use crate::light_map::{LightRaster, TEXELS_PER_TILE};
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
    #[sampler(1)]
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

/// The last light worked out, kept so moving about only redoes what the player sees.
#[derive(Resource, Default)]
struct LightCache {
    digest: Option<u64>,
    sight: Option<u64>,
    z: i32,
    raster: LightRaster,
    /// The raster's light per texel, its shadow edges softened.
    soft: Vec<[f32; 3]>,
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
    (screens, glow): (
        Query<(&GridPos, &crate::sprites::Look), (Without<LightSource>, Without<Hidden>)>,
        Option<Res<crate::glow::GlowAtlas>>,
    ),
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

    let mut emitters: Vec<(TilePos, [f32; 3])> = sources
        .iter()
        .filter(|(pos, _)| pos.0.z == map.z)
        .map(|(pos, LightSource(rgb))| (pos.0, rgb.map(f32::from)))
        .collect();
    // Glowing screens cast a faint light of their own colour (`glow.rs`).
    if let Some(glow) = &glow {
        emitters.extend(
            screens
                .iter()
                .filter(|(pos, _)| pos.0.z == map.z)
                .filter_map(|(pos, look)| Some((pos.0, glow.screen_light(&look.sprite, &look.state)?))),
        );
    }
    emitters.sort_by_key(|(pos, _)| *pos);
    let mut opaque: Vec<TilePos> = doors.iter().map(|p| p.0).collect();
    opaque.sort();
    let blocks = |p: TilePos| map.is_opaque(p) || opaque.binary_search(&p).is_ok();

    let mut digest = std::collections::hash_map::DefaultHasher::new();
    map.z.hash(&mut digest);
    for m in &maps {
        m.last_changed().hash(&mut digest);
    }
    for (pos, rgb) in &emitters {
        pos.hash(&mut digest);
        rgb.map(f32::to_bits).hash(&mut digest);
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
    if cache.digest != Some(digest)
        || cache.z != map.z
        || cache.raster.width != width as usize
        || cache.raster.height != height as usize
    {
        let mut raster = LightRaster::new(width as usize, height as usize);
        let opaque_at = |x: i32, y: i32| blocks(TilePos::new(x, y, map.z));
        for (pos, rgb) in &emitters {
            raster.add_light(pos.x, pos.y, *rgb, &opaque_at);
        }
        // Space is always fully lit (Luna's space area has no lighting).
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if map.tile(TilePos::new(x, y, map.z)).is_some_and(|t| t.id == SPACE) {
                    raster.light_fully(x, y);
                }
            }
        }
        cache.soft = raster.softened();
        cache.raster = raster;
        cache.digest = Some(digest);
        cache.z = map.z;
    }
    cache.sight = Some(view);

    // Tiles whose centre our eye can't see, within the view: what lies on them isn't drawn.
    // The darkness itself is drawn as shadows (`shadows.rs`).
    let mut unseen = HashSet::new();
    if let Some((pos, _)) = me {
        let eye = Vec2::new(pos.0.x as f32, pos.0.y as f32) * TILE_PX;
        let walls = crate::shadows::opaque_near(map, pos.0, opaque.iter().copied());
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let tile = TilePos::new(x, y, map.z);
                let far = (tile.x - pos.0.x).abs() > VIEW_RANGE || (tile.y - pos.0.y).abs() > VIEW_RANGE;
                if far || !crate::shadows::sees(eye, tile, &walls) {
                    unseen.insert(tile);
                }
            }
        }
    }

    // Walls and closed doors show the light on the side the player sees.
    let eye = me.map(|(p, _)| [p.0.x as f32, p.0.y as f32]);
    let raster = &cache.raster;
    let inside = |p: TilePos| (0..width as i32).contains(&p.x) && (0..height as i32).contains(&p.y);
    let seen = |p: TilePos| inside(p) && (me.is_none() || !unseen.contains(&p));
    let walls: std::collections::HashMap<(i32, i32), [f32; 3]> = (0..height as i32)
        .flat_map(|y| (0..width as i32).map(move |x| (x, y)))
        .filter(|(x, y)| blocks(TilePos::new(*x, *y, map.z)))
        .map(|(x, y)| {
            let light = wall_light(TilePos::new(x, y, map.z), eye, |p| raster.at(p), blocks, seen);
            ((x, y), light.unwrap_or([0.0; 3]))
        })
        .collect();

    let k = TEXELS_PER_TILE;
    let (tw, th) = (width as usize * k, height as usize * k);
    let mut data = Vec::with_capacity(tw * th * 4);
    // Texture rows run from the top of the map down.
    for row in 0..th {
        let j = th - 1 - row;
        for i in 0..tw {
            let tile = ((i / k) as i32, (j / k) as i32);
            let level = walls.get(&tile).copied().unwrap_or(cache.soft[j * tw + i]);
            for c in level.map(|c| c.max(MIN_LIGHT)) {
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
            width: tw as u32,
            height: th as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    // Blended smoothly between texels, a sixteenth of a tile apart.
    image.sampler = ImageSampler::linear();
    let image = images.add(image);
    let size = Vec2::new(width as f32, height as f32) * TILE_PX;
    // Tile (x, y) is centred on (x, y) tiles: the texture spans from half a tile before the
    // first to half a tile past the last.
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

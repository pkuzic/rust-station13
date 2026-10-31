//! Drawing with NSV Luna's sprites: the embedded atlas, animated looks, wall smoothing,
//! cable and pipe joins, and people's clothes and facing.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use rs13_dmi_import::atlas::Atlas;
use rs13_world::{Appearance, GridPos, NetId, Slot, TileMap, TilePos, WornView};

/// Built by `rs13-dmi-import` from `client-assets/sprites.ron`. Embedded until assets are
/// delivered by the server's asset manifest.
const ATLAS_PNG: &[u8] = include_bytes!("../../../client-assets/sprites/atlas.png");
const ATLAS_RON: &str = include_str!("../../../client-assets/sprites/atlas.ron");

#[derive(Resource)]
pub struct SpriteAtlas {
    pub atlas: Atlas,
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    /// The atlas as decoded, kept to enlarge for the UI.
    source: Image,
    /// For the HUD and panels: the atlas enlarged to the screen's pixel scale and sampled
    /// smoothly (`scaling.rs`). Both handles stay the same when the scale changes.
    ui_image: Handle<Image>,
    ui_layout: Handle<TextureAtlasLayout>,
}

/// What an entity looks like, by atlas key. `facing` is a DMI direction index
/// (0 south, 1 north, 2 east, 3 west).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Look {
    pub sprite: String,
    pub state: String,
    pub facing: u32,
    /// When the current state started, for animations (seconds).
    pub since: f32,
}

/// Clothing drawn on top of a person; copies the person's facing and posture.
#[derive(Component, Debug, Clone, Copy)]
pub struct Layer;

pub fn load_atlas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let atlas: Atlas = ron::from_str(ATLAS_RON).expect("built-in atlas description is valid");
    let image = Image::from_buffer(
        ATLAS_PNG,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::default(),
    )
    .expect("built-in atlas image is valid");
    let rows = image.height() / atlas.tile;
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(atlas.tile), atlas.columns, rows, None, None);
    let ui_image = crate::scaling::enlarged_image(&image, 1).expect("the built-in atlas has pixels");
    commands.insert_resource(SpriteAtlas {
        ui_image: images.add(ui_image),
        ui_layout: layouts.add(layout.clone()),
        source: image.clone(),
        image: images.add(image),
        layout: layouts.add(layout),
        atlas,
    });
}

impl SpriteAtlas {
    /// The atlas as decoded.
    pub fn source(&self) -> &Image {
        &self.source
    }

    /// The atlas image the map's sprites are drawn from.
    pub fn image(&self) -> Handle<Image> {
        self.image.clone()
    }

    /// Atlas cell of the first frame of `sprite:state` facing `facing`.
    pub fn index(&self, sprite: &str, state: &str, facing: u32) -> Option<usize> {
        Some(self.atlas.get(sprite, state)?.cell(0, facing) as usize)
    }

    /// A UI image of the first frame of `sprite:state`, if the atlas has it.
    pub fn ui_image(&self, sprite: &str, state: &str, facing: u32) -> Option<ImageNode> {
        Some(ImageNode::from_atlas_image(
            self.ui_image.clone(),
            TextureAtlas {
                layout: self.ui_layout.clone(),
                index: self.index(sprite, state, facing)?,
            },
        ))
    }

    /// Whether atlas cell `index` has a visible pixel at (`x`, `y`), counted from its top-left
    /// corner: what a click there lands on (BYOND clicks only opaque pixels).
    pub fn opaque_at(&self, index: usize, x: u32, y: u32) -> bool {
        let tile = self.atlas.tile;
        if x >= tile || y >= tile {
            return false;
        }
        let Some(data) = self.source.data.as_ref() else {
            return false;
        };
        let columns = self.atlas.columns.max(1) as usize;
        let (col, row) = (index % columns, index / columns);
        let width = self.source.width() as usize;
        let (px, py) = (col * tile as usize + x as usize, row * tile as usize + y as usize);
        data.get((py * width + px) * 4 + 3).is_some_and(|a| *a > 0)
    }

    /// Enlarge the UI's copy of the atlas `k` times, in place.
    pub fn enlarge_ui(&self, k: u32, images: &mut Assets<Image>, layouts: &mut Assets<TextureAtlasLayout>) {
        if let (Some(enlarged), Some(mut image)) = (
            crate::scaling::enlarged_image(&self.source, k),
            images.get_mut(&self.ui_image),
        ) {
            *image = enlarged;
        }
        if let Some(mut layout) = layouts.get_mut(&self.ui_layout) {
            let rows = self.source.height() / self.atlas.tile;
            *layout = TextureAtlasLayout::from_grid(
                UVec2::splat(self.atlas.tile * k),
                self.atlas.columns,
                rows,
                None,
                None,
            );
        }
    }

    /// A sprite showing the first frame of `sprite:state`, if the atlas has it.
    pub fn sprite(&self, sprite: &str, state: &str, facing: u32) -> Option<Sprite> {
        let anim = self.atlas.get(sprite, state)?;
        Some(Sprite::from_atlas_image(
            self.image.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index: anim.cell(0, facing) as usize,
            },
        ))
    }
}

/// Atlas state for an object's server-side appearance. Overlays after `+` (as in
/// `closed+panel`) are drawn by `overlays`.
pub fn state_for(appearance: &Appearance) -> String {
    let base = appearance.state.split('+').next().unwrap_or_default();
    match (appearance.sprite.as_str(), base) {
        ("human", "unconscious" | "dead") => "lying".to_owned(),
        // Until `join_lines` sees the neighbours, draw a straight east–west run.
        ("cable" | "pipe", "") => "4-8".to_owned(),
        (_, state) => state.to_owned(),
    }
}

/// Wall sprites join with neighbouring walls: N 1 + S 2 + E 4 + W 8 (Luna's smoothing).
pub fn wall_state(map: &TileMap, pos: TilePos) -> String {
    let wall = |dx: i32, dy: i32| {
        map.tile(TilePos::new(pos.x + dx, pos.y + dy, pos.z))
            .is_some_and(|t| t.id == "wall" || t.id == "r-wall")
    };
    let junction = u8::from(wall(0, 1))
        | u8::from(wall(0, -1)) << 1
        | u8::from(wall(1, 0)) << 2
        | u8::from(wall(-1, 0)) << 3;
    junction.to_string()
}

/// Space shimmers in a few variants, picked by position so it never changes.
pub fn space_state(pos: TilePos) -> String {
    ((pos.x.wrapping_mul(31) ^ pos.y.wrapping_mul(17)).rem_euclid(5)).to_string()
}

/// Keep every look's atlas cell in step with its state, facing and animation clock.
pub fn animate(time: Res<Time>, atlas: Option<Res<SpriteAtlas>>, mut looks: Query<(&Look, &mut Sprite)>) {
    let Some(atlas) = atlas else { return };
    let now = time.elapsed_secs();
    for (look, mut sprite) in &mut looks {
        let Some(anim) = atlas.atlas.get(&look.sprite, &look.state) else {
            continue;
        };
        let frame = anim.frame_at((now - look.since) * 10.0);
        let index = anim.cell(frame, look.facing) as usize;
        if let Some(texture) = sprite.texture_atlas.as_mut()
            && texture.index != index
        {
            texture.index = index;
        }
    }
}

/// Clothing copies its wearer's facing and posture.
pub fn dress(people: Query<&Look, Without<Layer>>, mut layers: Query<(&ChildOf, &mut Look), With<Layer>>) {
    for (parent, mut look) in &mut layers {
        let Ok(person) = people.get(parent.parent()) else {
            continue;
        };
        if look.facing != person.facing || look.state != person.state {
            look.facing = person.facing;
            look.state = person.state.clone();
        }
    }
}

/// Cables and pipes join toward neighbouring pieces of the same network: the state is
/// the pair of directions (N 1, S 2, E 4, W 8), e.g. "4-8" for an east–west run.
pub fn join_lines(
    atlas: Option<Res<SpriteAtlas>>,
    added: Query<(), Added<Appearance>>,
    things: Query<(&GridPos, &Appearance)>,
    mut lines: Query<(&GridPos, &Appearance, &mut Look)>,
) {
    let Some(atlas) = atlas else { return };
    if added.is_empty() {
        return;
    }
    let links = |kind: &str, sprite: &str| match kind {
        "cable" => matches!(sprite, "cable" | "smes" | "apc"),
        _ => {
            matches!(
                sprite,
                "pipe" | "manifold" | "valve" | "pump" | "vent" | "scrubber" | "connector"
            ) || rs13_classic::machines::is_pipe_tank(sprite)
        }
    };
    for (pos, appearance, mut look) in &mut lines {
        let kind = appearance.sprite.as_str();
        if kind != "cable" && kind != "pipe" {
            continue;
        }
        // A cable, or a pipe placed with a direction, already says which sides it joins.
        if !appearance.state.is_empty() {
            continue;
        }
        let mut dirs: Vec<u8> = Vec::new();
        for (bit, dx, dy) in [(1u8, 0, 1), (2, 0, -1), (4, 1, 0), (8, -1, 0)] {
            let next = TilePos::new(pos.0.x + dx, pos.0.y + dy, pos.0.z);
            if things.iter().any(|(p, a)| p.0 == next && links(kind, &a.sprite)) {
                dirs.push(bit);
            }
        }
        let state = match (kind, dirs.as_slice()) {
            (_, [a, b]) => format!("{a}-{b}"),
            ("cable", [a]) => format!("0-{a}"),
            ("pipe", [1 | 2]) => "1-2".to_owned(),
            ("pipe", _) => "4-8".to_owned(),
            ("cable", [a, ..]) => format!("0-{a}"),
            _ => "0-1".to_owned(),
        };
        if atlas.atlas.get(kind, &state).is_some() && look.state != state {
            look.state = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> Atlas {
        ron::from_str(ATLAS_RON).unwrap()
    }

    #[test]
    fn every_sprite_the_game_uses_is_in_the_atlas() {
        let atlas = atlas();
        for sprite in [
            "crowbar",
            "wrench",
            "screwdriver",
            "wirecutters",
            "metal-sheet",
            "id-card",
            "oxygen-tank",
            "human",
            "smes",
            "apc",
            "connector",
            "air-tank",
            "canister",
        ] {
            assert!(atlas.get(sprite, "").is_some(), "{sprite}");
        }
        for state in ["closed", "opening", "open", "closing", "deny"] {
            assert!(atlas.get("airlock", state).is_some(), "airlock:{state}");
        }
        for sprite in ["vent", "scrubber", "pump"] {
            for state in ["on", "off"] {
                assert!(atlas.get(sprite, state).is_some(), "{sprite}:{state}");
            }
        }
        for junction in 0..16 {
            assert!(atlas.get("tile:wall", &junction.to_string()).is_some());
        }
        assert!(atlas.get("human", "lying").is_some());
    }

    #[test]
    fn walls_join_their_wall_neighbours() {
        let map = TileMap::from_ron(
            r#"(
                legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
                rows: ["...", "WWW", ".W."],
            )"#,
            0,
        )
        .unwrap();
        // Middle of the row: walls east (4) and west (8) and south (2).
        assert_eq!(wall_state(&map, TilePos::new(1, 1, 0)), "14");
        // Left end: only east.
        assert_eq!(wall_state(&map, TilePos::new(0, 1, 0)), "4");
        // Bottom: only north.
        assert_eq!(wall_state(&map, TilePos::new(1, 0, 0)), "1");
    }

    #[test]
    fn space_variants_are_stable_and_in_range() {
        for x in -5..5 {
            for y in -5..5 {
                let s: u32 = space_state(TilePos::new(x, y, 0)).parse().unwrap();
                assert!(s < 5);
                assert_eq!(space_state(TilePos::new(x, y, 0)), s.to_string());
            }
        }
    }
}

/// The atlas keys a person's clothing layers were last built from, bottom to top.
#[derive(Component, Debug, Default, PartialEq)]
pub struct Dressed(Vec<String>);

/// Order clothing is drawn over the body, bottom to top (Luna's overlays).
const DRAW_ORDER: [Slot; 11] = [
    Slot::Uniform,
    Slot::Id,
    Slot::Shoes,
    Slot::Gloves,
    Slot::Belt,
    Slot::Suit,
    Slot::Back,
    Slot::Glasses,
    Slot::Ears,
    Slot::Mask,
    Slot::Head,
];

/// Rebuild a person's clothing layers whenever what they wear changes. A worn item may
/// replicate a frame after the person's `WornView`; it is picked up once it arrives.
pub fn wear_layers(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    people: Query<(Entity, &WornView, &Look, Option<&Dressed>, Option<&Children>), Without<Layer>>,
    items: Query<(&NetId, &Appearance)>,
    layers: Query<(), With<Layer>>,
) {
    let Some(atlas) = atlas else { return };
    for (person, worn, look, dressed, children) in &people {
        let keys: Vec<String> = DRAW_ORDER
            .into_iter()
            .filter_map(|slot| worn.get(slot))
            .filter_map(|id| items.iter().find(|(n, _)| **n == id).map(|(_, a)| a))
            // An item in a state of its own (a lit hard hat) is drawn as
            // "worn:<sprite>-<state>" when there is one.
            .map(|a| {
                let stated = format!("{}-{}", a.sprite, a.state);
                if !a.state.is_empty() && atlas.atlas.get(&format!("worn:{stated}"), "").is_some() {
                    stated
                } else {
                    a.sprite.clone()
                }
            })
            .flat_map(|sprite| [format!("worn:{sprite}"), format!("worn:{sprite}:l")])
            .filter(|key| atlas.atlas.get(key, "").is_some())
            .collect();
        if dressed.is_some_and(|d| d.0 == keys) {
            continue;
        }
        for child in children.into_iter().flatten() {
            if layers.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        let mut e = commands.entity(person);
        for (i, key) in keys.iter().enumerate() {
            if let Some(image) = atlas.sprite(key, &look.state, look.facing) {
                e.with_child((
                    image,
                    // Slightly in front of the body, in the parent's space.
                    Transform::from_xyz(0.0, 0.0, 0.01 * (i + 1) as f32),
                    Look {
                        sprite: key.clone(),
                        state: look.state.clone(),
                        facing: look.facing,
                        since: 0.0,
                    },
                    Layer,
                ));
            }
        }
        e.insert(Dressed(keys));
    }
}

/// An overlay drawn on top of its parent, from the parent's `+state` suffixes.
#[derive(Component, Debug, Clone, Copy)]
pub struct OverlayPart;

/// The overlay states an object's overlays were last built from.
#[derive(Component, Debug, Default, PartialEq)]
pub struct Overlaid(Vec<String>);

/// Keep each object's overlay children in step with its appearance: `closed+panel` draws
/// the `panel` state of the same sprite over the door (Luna `overlays`).
pub fn overlays(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    things: Query<(Entity, &Appearance, Option<&Overlaid>, Option<&Children>), With<Look>>,
    parts: Query<(), With<OverlayPart>>,
) {
    let Some(atlas) = atlas else { return };
    for (entity, appearance, overlaid, children) in &things {
        let wanted: Vec<String> = appearance.state.split('+').skip(1).map(str::to_owned).collect();
        if overlaid.map_or(wanted.is_empty(), |o| o.0 == wanted) {
            continue;
        }
        for child in children.into_iter().flatten() {
            if parts.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        let mut e = commands.entity(entity);
        for (i, state) in wanted.iter().enumerate() {
            if let Some(image) = atlas.sprite(&appearance.sprite, state, 0) {
                e.with_child((
                    image,
                    Transform::from_xyz(0.0, 0.0, 0.001 * (i + 1) as f32),
                    OverlayPart,
                ));
            }
        }
        e.insert(Overlaid(wanted));
    }
}

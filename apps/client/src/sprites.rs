//! Drawing with NSV Luna's sprites: the embedded atlas, animated looks, wall smoothing,
//! cable and pipe joins, and people's clothes and facing.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use rs13_dmi_import::atlas::Atlas;
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

/// Built by `rs13-dmi-import` from `client-assets/sprites.ron`. Embedded until assets are
/// delivered by the server's asset manifest.
const ATLAS_PNG: &[u8] = include_bytes!("../../../client-assets/sprites/atlas.png");
const ATLAS_RON: &str = include_str!("../../../client-assets/sprites/atlas.ron");

#[derive(Resource)]
pub struct SpriteAtlas {
    pub atlas: Atlas,
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
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
    commands.insert_resource(SpriteAtlas {
        atlas,
        image: images.add(image),
        layout: layouts.add(layout),
    });
}

impl SpriteAtlas {
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

/// Atlas state for an object's server-side appearance.
pub fn state_for(appearance: &Appearance) -> String {
    match (appearance.sprite.as_str(), appearance.state.as_str()) {
        ("human", "unconscious" | "dead") => "lying".to_owned(),
        // Until `join_lines` sees the neighbours, draw a straight east–west run.
        ("cable" | "pipe", _) => "4-8".to_owned(),
        (_, state) => state.to_owned(),
    }
}

/// Wall sprites join with neighbouring walls: N 1 + S 2 + E 4 + W 8 (Luna's smoothing).
pub fn wall_state(map: &TileMap, pos: TilePos) -> String {
    let wall = |dx: i32, dy: i32| {
        map.tile(TilePos::new(pos.x + dx, pos.y + dy, pos.z))
            .is_some_and(|t| t.id == "wall")
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
        _ => matches!(sprite, "pipe" | "valve" | "pump" | "air-tank"),
    };
    for (pos, appearance, mut look) in &mut lines {
        let kind = appearance.sprite.as_str();
        if kind != "cable" && kind != "pipe" {
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

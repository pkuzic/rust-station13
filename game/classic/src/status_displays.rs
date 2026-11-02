//! Status displays: the frame on the wall, and up to two lines of five characters drawn
//! with Luna's glyphs. Spec: `docs/reference/status-displays.md`.
//!
//! At round start Luna's displays show the evacuation shuttle's countdown, which is
//! blank until a shuttle is called, and the two supply displays show where the supply
//! shuttle is. There is no shuttle yet: the first stay blank, the supply ones read
//! "SPPLY" over "AWAY".

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, PixelOffset, TilePos};

use crate::power::{Channel, PowerUser};

pub const STATUS_DISPLAY: &str = "status-display";
/// The sprite each character is drawn with (its state is the character).
pub const GLYPH: &str = "status-glyph";
/// Its draw, W (Luna `idle_power_usage = 10`).
pub const DISPLAY_POWER: f64 = 10.0;
/// Characters a line holds (Luna: five glyphs of 5 px).
pub const LINE_CHARS: usize = 5;

/// A status display, and the two lines it shows.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct StatusDisplay {
    pub lines: (String, Option<String>),
    /// The glyphs it has drawn.
    pub glyphs: Vec<Entity>,
}

/// On a glyph: the display it belongs to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph(pub Entity);

pub fn spawn_display(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    // Luna's supply displays (`mode = 4`) show where the supply shuttle is: away.
    let supply = spec.vars.get("mode").is_some_and(|m| m.trim() == "4");
    let lines = if supply {
        ("SPPLY".to_owned(), Some("AWAY".to_owned()))
    } else {
        (String::new(), None)
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(STATUS_DISPLAY, "frame"),
            StatusDisplay {
                lines,
                glyphs: Vec::new(),
            },
            PowerUser::new(Channel::Equipment, DISPLAY_POWER),
        ))
        .id()
}

/// Where each character of `lines` goes, in pixels from the tile (Luna `texticon`: right to
/// left from 23 px, 5 px apart; two lines at -9 and -17, or one at -13), with its glyph.
pub fn layout(lines: &(String, Option<String>)) -> Vec<(char, [i8; 2])> {
    let mut out = Vec::new();
    let mut line = |text: &str, y: i8| {
        let chars: Vec<char> = text.chars().take(LINE_CHARS).collect();
        for (from_end, c) in chars.iter().rev().enumerate() {
            if *c != ' ' {
                out.push((*c, [23 - 5 * from_end as i8, y]));
            }
        }
    };
    match &lines.1 {
        Some(second) => {
            line(&lines.0, -9);
            line(second, -17);
        }
        None => line(&lines.0, -13),
    }
    out
}

/// Draw each display's lines while it has power, and clear them without (Luna `process`).
pub fn update_displays(
    mut displays: Query<(Entity, &GridPos, &mut StatusDisplay, &PowerUser)>,
    mut ids: ResMut<crate::objects::NetIds>,
    mut commands: Commands,
) {
    for (entity, pos, mut display, power) in &mut displays {
        let want = if power.powered {
            layout(&display.lines)
        } else {
            Vec::new()
        };
        if display.glyphs.len() == want.len() {
            continue;
        }
        for glyph in display.glyphs.drain(..) {
            commands.entity(glyph).despawn();
        }
        for (c, offset) in want {
            let glyph = commands
                .spawn((
                    GridPos(pos.0),
                    ids.alloc(),
                    Appearance::new(GLYPH, c.to_string()),
                    PixelOffset(offset),
                    Glyph(entity),
                ))
                .id();
            display.glyphs.push(glyph);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_lines_right_aligned_five_pixels_apart() {
        let glyphs = layout(&("SPPLY".to_owned(), Some("AWAY".to_owned())));
        assert_eq!(glyphs[0], ('Y', [23, -9]));
        assert_eq!(glyphs[4], ('S', [3, -9]));
        assert_eq!(glyphs[5], ('Y', [23, -17]));
        assert_eq!(glyphs[8], ('A', [8, -17]));
        assert!(layout(&(String::new(), None)).is_empty());
    }
}

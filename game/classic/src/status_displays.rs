//! Status displays: the frame on the wall, and up to two lines of five characters drawn
//! with Luna's glyphs. Spec: `docs/reference/status-displays.md`.
//!
//! Luna's displays show the countdown to the escape pods, which is blank until they're called, or
//! whatever a communications console sets. The two supply displays show where the supply shuttle
//! is. There is no shuttle yet, so they read "SPPLY" over "AWAY".

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, PixelOffset, TilePos};

use crate::escape::{DisplayMode, EscapePods, PodState, StationAlert};
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
    /// What those glyphs are.
    drawn: Vec<(String, [i8; 2])>,
    /// A supply display, which the console doesn't change.
    supply: bool,
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
                drawn: Vec::new(),
                supply,
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

/// What the ordinary displays should show now: a picture, or two lines (Luna's `update`, `get_shuttle_timer`).
fn what_to_show(
    alert: &StationAlert,
    pods: &EscapePods,
    now: f32,
) -> (Option<&'static str>, (String, Option<String>)) {
    let timer = |label: &str, at: f32| {
        let seconds = ((at - now) / 10.0).ceil().max(0.0) as u32;
        (
            label.to_owned(),
            Some(format!("{:02}:{:02}", seconds / 60, seconds % 60)),
        )
    };
    match &alert.display {
        DisplayMode::Blank => (None, (String::new(), None)),
        DisplayMode::Picture(picture) => (Some(*picture), (String::new(), None)),
        DisplayMode::Message(one, two) => {
            let two = (!two.is_empty()).then(|| two.to_uppercase());
            (None, (one.to_uppercase(), two))
        }
        DisplayMode::Pods => match pods.state {
            PodState::Launching { at, .. } => (None, timer("ETL", at)),
            PodState::Transit { at } => (None, timer("ETA", at)),
            PodState::Docked | PodState::Arrived => (None, (String::new(), None)),
        },
    }
}

/// Draw each display's lines while it has power, and clear them without (Luna `process`).
pub fn update_displays(
    alert: Res<StationAlert>,
    pods: Res<EscapePods>,
    time: Res<crate::time::WorldTime>,
    mut displays: Query<(Entity, &GridPos, &mut StatusDisplay, &PowerUser)>,
    mut ids: ResMut<crate::objects::NetIds>,
    mut commands: Commands,
) {
    let (picture, lines) = what_to_show(&alert, &pods, time.deciseconds());
    for (entity, pos, mut display, power) in &mut displays {
        if !display.supply && display.lines != lines {
            display.lines = lines.clone();
        }
        let shows_picture = picture.filter(|_| !display.supply);
        let want: Vec<(String, [i8; 2])> = if !power.powered {
            Vec::new()
        } else if let Some(picture) = shows_picture {
            vec![(picture.to_owned(), [0, 0])]
        } else {
            layout(&display.lines)
                .into_iter()
                .map(|(c, at)| (c.to_string(), at))
                .collect()
        };
        if display.drawn == want {
            continue;
        }
        for glyph in display.glyphs.drain(..) {
            commands.entity(glyph).despawn();
        }
        display.drawn.clone_from(&want);
        for (c, offset) in want {
            let glyph = commands
                .spawn((
                    GridPos(pos.0),
                    ids.alloc(),
                    Appearance::new(GLYPH, c),
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

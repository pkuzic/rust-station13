//! The classic HUD from Luna's `human_hud`, drawn with its `screen1.dmi` icons.
//!
//! The screen is a square of 16 × 16 cells: the 15 × 15 map view, a bar of buttons
//! along the bottom (row 0) and a column of indicators down the right (column 16).
//! Cells are placed by BYOND `screen_loc` coordinates: x 1–16 from the left and
//! y 0–15 from the bottom. Inventory slots overlap the map's bottom-left corner, as in Luna.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::Case;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel};
use rs13_world::{Appearance, Hand, HandsView, Intent, MobStat, NetId, Slot, WornView};

use crate::sprites::SpriteAtlas;
use crate::{Loc, OwnPlayer, OwnStatus};

/// Cells across and down the square screen.
pub const SCREEN_CELLS: f32 = 16.0;
/// One cell in `vmin`: the screen square is as large as the window's shorter side.
const CELL_VMIN: f32 = 100.0 / SCREEN_CELLS;

/// What a HUD cell is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Slot(Slot),
    Hand(Hand),
    SwapHands,
    Drop,
    Intent,
    MoveIntent,
    Internals,
    Oxygen,
    Toxin,
    Fire,
    Temperature,
    Health,
}

/// Luna's layout (`code/game/hud.dm` `ui_*`): cell, screen x, screen y.
const LAYOUT: [(Cell, u8, u8); 21] = [
    (Cell::Slot(Slot::Mask), 1, 2),
    (Cell::Slot(Slot::Head), 2, 2),
    (Cell::Slot(Slot::Back), 3, 2),
    // The right hand is on the left of the screen: you face the player.
    (Cell::Hand(Hand::Right), 1, 1),
    (Cell::Slot(Slot::Suit), 2, 1),
    (Cell::Hand(Hand::Left), 3, 1),
    (Cell::Slot(Slot::Shoes), 4, 1),
    (Cell::Slot(Slot::Gloves), 5, 1),
    (Cell::Slot(Slot::Id), 1, 0),
    (Cell::Slot(Slot::Uniform), 2, 0),
    (Cell::Slot(Slot::Belt), 3, 0),
    (Cell::SwapHands, 6, 0),
    (Cell::Drop, 7, 0),
    (Cell::Intent, 12, 0),
    (Cell::MoveIntent, 14, 0),
    (Cell::Internals, 16, 13),
    (Cell::Oxygen, 16, 11),
    (Cell::Toxin, 16, 9),
    (Cell::Fire, 16, 7),
    (Cell::Temperature, 16, 5),
    (Cell::Health, 16, 4),
];

/// A clickable HUD cell; its `ImageNode` is the frame or indicator.
#[derive(Component)]
pub struct HudCell(pub Cell);

/// The item drawn inside a slot or hand cell.
#[derive(Component)]
pub struct CellItem(Cell);

/// Walking instead of running, toggled with the move-intent button. Shift inverts it.
#[derive(Resource, Default)]
pub struct WalkMode(pub bool);

/// What the pointer is over on the HUD, shown in the stat panel.
#[derive(Resource, Default)]
pub struct HudTip(pub String);

pub fn place(x: u8, y: u8) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::VMin((f32::from(x) - 1.0) * CELL_VMIN),
        top: Val::VMin((SCREEN_CELLS - 1.0 - f32::from(y)) * CELL_VMIN),
        width: Val::VMin(CELL_VMIN),
        height: Val::VMin(CELL_VMIN),
        ..default()
    }
}

/// The frame or indicator icon a cell shows right now.
fn cell_state(
    cell: Cell,
    hands: Option<&HandsView>,
    status: Option<&rs13_network::protocol::StatusUpdate>,
    walk: bool,
) -> String {
    let flag = |on: bool| u8::from(on);
    let alert = |a: rs13_world::EnvAlert| status.is_some_and(|s| s.alerts.contains(&a));
    match cell {
        Cell::Slot(Slot::Mask) => "equip-n".into(),
        Cell::Slot(Slot::Head) => "hair".into(),
        Cell::Slot(Slot::Back) => "equip-ne".into(),
        Cell::Slot(Slot::Suit) => "equip-s".into(),
        Cell::Slot(Slot::Shoes) => "shoes".into(),
        Cell::Slot(Slot::Gloves) => "gloves".into(),
        Cell::Slot(Slot::Id) => "equip-sw".into(),
        Cell::Slot(Slot::Uniform) => "center".into(),
        Cell::Slot(Slot::Belt) => "belt".into(),
        Cell::Hand(Hand::Right) => "equip-w".into(),
        Cell::Hand(Hand::Left) => "equip-e".into(),
        Cell::SwapHands => match hands.map(|h| h.active) {
            Some(Hand::Left) => "hand-l".into(),
            _ => "hand-r".into(),
        },
        Cell::Drop => "drop".into(),
        Cell::Intent => match status.map(|s| s.intent) {
            Some(Intent::Harm) => "harm".into(),
            _ => "help".into(),
        },
        Cell::MoveIntent => if walk { "walking" } else { "running" }.into(),
        Cell::Internals => format!("internal{}", flag(status.is_some_and(|s| s.internals))),
        Cell::Oxygen => format!("oxy{}", flag(alert(rs13_world::EnvAlert::LowOxygen))),
        Cell::Toxin => "tox0".into(),
        Cell::Fire => format!("fire{}", flag(alert(rs13_world::EnvAlert::Hot))),
        Cell::Temperature => temperature_state(status.map_or(310, |s| s.body_temperature)).into(),
        Cell::Health => health_state(status).into(),
    }
}

/// Luna's body temperature icon (`human_life.dm`), 310 K being normal.
pub fn temperature_state(kelvin: u16) -> &'static str {
    match kelvin {
        370.. => "temp4",
        350..370 => "temp3",
        335..350 => "temp2",
        320..335 => "temp1",
        300..320 => "temp0",
        295..300 => "temp-1",
        285..295 => "temp-2",
        280..285 => "temp-3",
        _ => "temp-4",
    }
}

/// Luna's health doll: 0 healthy … 6 in crit, 7 dead.
fn health_state(status: Option<&rs13_network::protocol::StatusUpdate>) -> &'static str {
    let Some(status) = status else { return "health0" };
    if status.stat == MobStat::Dead {
        return "health7";
    }
    match status.health {
        100.. => "health0",
        80..100 => "health1",
        60..80 => "health2",
        40..60 => "health3",
        20..40 => "health4",
        0..20 => "health5",
        _ => "health6",
    }
}

pub fn spawn_hud(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    let root = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::VMin(100.0),
            height: Val::VMin(100.0),
            ..default()
        })
        .id();
    let mut frame = |x: u8, y: u8, state: &str| {
        if let Some(image) = atlas.ui_image("hud", state, 0) {
            commands.spawn((place(x, y), image, Pickable::IGNORE, ChildOf(root)));
        }
    };
    // Bar backgrounds: along the bottom, down the right, and the corner.
    for x in 1..=15 {
        frame(x, 0, "bar-e");
    }
    for y in 1..=15 {
        frame(16, y, "bar-s");
    }
    frame(16, 0, "bar-nw");

    for (cell, x, y) in LAYOUT {
        let state = cell_state(cell, None, None, false);
        let Some(image) = atlas.ui_image("hud", &state, 0) else {
            continue;
        };
        let mut e = commands.spawn((
            HudCell(cell),
            place(x, y),
            image,
            Hovered::default(),
            ChildOf(root),
        ));
        if matches!(cell, Cell::Slot(_) | Cell::Hand(_)) {
            e.with_child((
                CellItem(cell),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                // Any atlas cell: `update_hud` points it at the item.
                atlas.ui_image("hud", "center", 0).unwrap_or_default(),
                Visibility::Hidden,
                Pickable::IGNORE,
            ));
        }
        e.observe(click_cell);
    }
}

/// Keep frames, indicators, items in slots and the hover tip current.
pub fn update_hud(
    loc: Res<Loc>,
    atlas: Res<SpriteAtlas>,
    status: Res<OwnStatus>,
    walk: Res<WalkMode>,
    mut tip: ResMut<HudTip>,
    own: Query<(&HandsView, &WornView), With<OwnPlayer>>,
    items: Query<(&NetId, &Appearance)>,
    mut cells: Query<(&HudCell, &Hovered, &mut ImageNode), Without<CellItem>>,
    mut contents: Query<(&CellItem, &mut ImageNode, &mut Visibility)>,
) {
    let (hands, worn) = own.single().ok().unzip();
    let status = status.0.as_ref();
    let sprite_of = |id: Option<NetId>| {
        id.and_then(|id| items.iter().find(|(n, _)| **n == id))
            .map(|(_, a)| a.sprite.clone())
    };
    let content = |cell: Cell| match cell {
        Cell::Slot(slot) => sprite_of(worn.and_then(|w| w.get(slot))),
        Cell::Hand(Hand::Left) => sprite_of(hands.and_then(|h| h.left)),
        Cell::Hand(Hand::Right) => sprite_of(hands.and_then(|h| h.right)),
        _ => None,
    };

    let mut hovered = None;
    for (HudCell(cell), hover, mut image) in &mut cells {
        let state = cell_state(*cell, hands, status, walk.0);
        if let Some(index) = atlas.index("hud", &state, 0) {
            set_index(&mut image, index);
        }
        if hover.0 {
            hovered = Some(*cell);
        }
    }
    for (CellItem(cell), mut image, mut visibility) in &mut contents {
        let index = content(*cell).and_then(|sprite| atlas.index(&sprite, "", 0));
        match index {
            Some(index) => {
                set_index(&mut image, index);
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }

    let new_tip = hovered.map(|cell| {
        let name = cell_name(&loc, cell);
        match content(cell) {
            Some(sprite) => format!("{name}: {}", loc.0.noun(&sprite, Case::Nom)),
            None => name,
        }
    });
    let new_tip = new_tip.unwrap_or_default();
    if tip.0 != new_tip {
        tip.0 = new_tip;
    }
}

fn hand_item(hands: &HandsView, hand: Hand) -> Option<NetId> {
    match hand {
        Hand::Left => hands.left,
        Hand::Right => hands.right,
    }
}

fn set_index(image: &mut ImageNode, index: usize) {
    if let Some(texture) = image.texture_atlas.as_mut()
        && texture.index != index
    {
        texture.index = index;
    }
}

fn cell_name(loc: &Loc, cell: Cell) -> String {
    let mut args = rs13_i18n::Args::new();
    let id = match cell {
        Cell::Slot(slot) => {
            args.set("slot", slot.id());
            return loc.0.format("hud-slot", Some(&args)).unwrap_or_default();
        }
        Cell::Hand(Hand::Left) => "hand-left",
        Cell::Hand(Hand::Right) => "hand-right",
        Cell::SwapHands => "swap",
        Cell::Drop => "drop",
        Cell::Intent => "intent",
        Cell::MoveIntent => "move",
        Cell::Internals => "internals",
        Cell::Oxygen => "oxygen",
        Cell::Toxin => "toxin",
        Cell::Fire => "fire",
        Cell::Temperature => "temperature",
        Cell::Health => "health",
    };
    args.set("what", id);
    loc.0.format("hud-tip", Some(&args)).unwrap_or_default()
}

/// Luna's HUD clicks. A slot puts on the active-hand item, or with an empty hand takes
/// off what is there; the inactive hand becomes active. The server checks the rules.
fn click_cell(
    click: On<PointerClick>,
    cells: Query<&HudCell>,
    status: Res<OwnStatus>,
    mut walk: ResMut<WalkMode>,
    own: Query<(&HandsView, &WornView), With<OwnPlayer>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(HudCell(cell)) = cells.get(click.entity) else {
        return;
    };
    if *cell == Cell::MoveIntent {
        walk.0 = !walk.0;
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let (hands, worn) = own.single().ok().unzip();
    let holding = hands.is_some_and(|h| hand_item(h, h.active).is_some());
    let action = match *cell {
        // With an empty hand, clicking what you wear is clicking the thing itself: a
        // container opens, clothes come off (Luna `attack_hand`).
        Cell::Slot(slot) => match worn.and_then(|w| w.get(slot)) {
            _ if holding => Some(ActionInput::Equip(slot)),
            Some(item) => Some(ActionInput::UseOn(item)),
            None => None,
        },
        // The active hand: use its item (Luna `attack_self`, e.g. light a welding tool).
        Cell::Hand(hand) if hands.is_some_and(|h| h.active == hand) => {
            hands.and_then(|h| hand_item(h, hand)).map(ActionInput::UseOn)
        }
        // The other hand: with the active one empty, its item (a container opens, anything
        // else moves over); otherwise just swap.
        Cell::Hand(hand) => hands.map(|h| match hand_item(h, hand) {
            Some(item) if !holding => ActionInput::UseOn(item),
            _ => ActionInput::SwapHands,
        }),
        Cell::SwapHands => Some(ActionInput::SwapHands),
        Cell::Drop => Some(ActionInput::Drop),
        Cell::Intent => Some(ActionInput::SetIntent(
            match status.0.as_ref().map(|s| s.intent) {
                Some(Intent::Harm) => Intent::Help,
                _ => Intent::Harm,
            },
        )),
        Cell::Internals => Some(ActionInput::ToggleInternals),
        _ => None,
    };
    if let Some(action) = action {
        sender.send::<IntentChannel>(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luna_temperature_bands() {
        assert_eq!(temperature_state(310), "temp0");
        assert_eq!(temperature_state(296), "temp-1");
        assert_eq!(temperature_state(250), "temp-4");
        assert_eq!(temperature_state(400), "temp4");
    }

    #[test]
    fn cells_do_not_overlap() {
        for (i, (_, x, y)) in LAYOUT.iter().enumerate() {
            assert!(LAYOUT[i + 1..].iter().all(|(_, x2, y2)| (x, y) != (x2, y2)));
        }
    }
}

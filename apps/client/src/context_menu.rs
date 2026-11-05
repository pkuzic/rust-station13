//! Right-click menu (BYOND's context menu): everything on the clicked tile, topmost first,
//! each with what can be done to it: examine it, use the active hand on it (a left click),
//! or pull it. Picking one sends that action; a left click elsewhere or Escape closes it.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel};
use rs13_world::{Appearance, DisplayName, GridPos, Hidden, MapLayer, NetId, PixelOffset, TilePos};

use crate::shadows::Sight;
use crate::{Loc, OwnPlayer, TILE_PX, WorldCamera, click_rank, cursor_tile, ui_text_font};

/// Things on one tile listed at most, besides the tile itself.
const MAX_THINGS: usize = 8;

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);

/// The open menu.
#[derive(Component)]
pub struct ContextMenu;

/// One of the menu's choices, in the order they are listed.
#[derive(Component, Clone)]
pub struct MenuChoice {
    pub index: usize,
    pub action: ActionInput,
}

/// Menu requests from scripted steps (`--use menu:DX,DY`, `pick:N`): open the menu on a
/// tile, or pick its `N`th choice (from 1).
#[derive(Resource, Default)]
pub struct MenuRequest {
    pub open_at: Option<(i32, i32)>,
    pub pick: Option<usize>,
}

/// The tile something is drawn on: its own, unless it's hung far enough over to show on the next
/// (BYOND `pixel_x`, `pixel_y`).
fn drawn_tile(pos: TilePos, offset: Option<&PixelOffset>) -> (i32, i32) {
    let shift = |d: i8| (f32::from(d) / TILE_PX).round() as i32;
    let (dx, dy) = offset.map_or((0, 0), |o| (shift(o.0[0]), shift(o.0[1])));
    (pos.x + dx, pos.y + dy)
}

/// Open the menu on the tile under the cursor on a right click, or on a scripted request.
#[allow(clippy::too_many_arguments)]
pub fn open_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<WorldCamera>>,
    loc: Res<Loc>,
    mut request: ResMut<MenuRequest>,
    own: Query<&GridPos, With<OwnPlayer>>,
    things: Query<
        (
            &NetId,
            &GridPos,
            &Appearance,
            Option<&DisplayName>,
            &Visibility,
            Has<OwnPlayer>,
            Option<&PixelOffset>,
        ),
        Without<Hidden>,
    >,
    menus: Query<Entity, With<ContextMenu>>,
    hud: Query<&Hovered>,
    maps: Query<&MapLayer>,
    sight: Res<Sight>,
    rev: Res<crate::rev_panel::RevMembers>,
) {
    let Ok(me) = own.single() else { return };
    let (tile, at) = if let Some((dx, dy)) = request.open_at.take() {
        let square = window.width().min(window.height());
        ((me.0.x + dx, me.0.y + dy), Vec2::new(square * 0.5, square * 0.5))
    } else if mouse.just_pressed(MouseButton::Right) && !hud.iter().any(|h| h.0) {
        let (Some(tile), Some(cursor)) = (cursor_tile(&window, &camera), window.cursor_position()) else {
            return;
        };
        (tile, cursor)
    } else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }

    let mut here: Vec<_> = things
        .iter()
        .filter(|(_, pos, a, _, visible, _, offset)| {
            pos.0.z == me.0.z
                && drawn_tile(pos.0, *offset) == tile
                && **visible != Visibility::Hidden
                && !matches!(a.sprite.as_str(), "fire" | "laser-beam" | "sparks" | "water-puff")
        })
        .collect();
    // The wall or floor itself, once it's in sight (BYOND lists the turf too).
    let tile_pos = TilePos::new(tile.0, tile.1, me.0.z);
    let turf = maps
        .iter()
        .find_map(|m| m.0.tile(tile_pos))
        .filter(|_| sight.sees(tile_pos))
        .map(|t| t.id.clone());
    if here.is_empty() && turf.is_none() {
        return;
    }
    here.sort_by_key(|(id, _, a, ..)| (click_rank(a), id.0));
    here.truncate(MAX_THINGS);

    let menu = commands
        .spawn((
            ContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(at.x),
                top: Val::Px(at.y),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(4.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the menu never reach the map under it.
            Hovered::default(),
            GlobalZIndex(20),
        ))
        .id();
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let mut index = 0;
    let noun_label = |noun: &str| {
        let mut args = rs13_i18n::Args::new();
        args.set("item", noun.to_owned());
        loc.0
            .format("menu-thing", Some(&args))
            .ok()
            .filter(|s| !s.contains('{'))
            .unwrap_or_else(|| noun.to_owned())
    };
    for (id, _, appearance, name, _, own, _) in here {
        let label = match name {
            Some(name) => name.0.clone(),
            None => noun_label(&appearance.sprite),
        };
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(menu),
            ))
            .id();
        commands.spawn((
            Text::new(label),
            ui_text_font(13.0),
            TextColor(Color::BLACK),
            Node {
                min_width: Val::Px(120.0),
                ..default()
            },
            ChildOf(row),
        ));
        let mut choices = vec![("menu-examine".to_owned(), ActionInput::Examine(*id))];
        if !own {
            choices.push(("menu-use".to_owned(), ActionInput::UseOn(*id)));
            choices.push(("menu-pull".to_owned(), ActionInput::Pull(*id)));
            // Its own verbs (`rs13_classic::verbs`).
            if !rev.0.is_empty() && matches!(appearance.sprite.as_str(), "human" | "human-f") {
                choices.push((
                    format!("verb-{}", rs13_classic::verbs::REV_CONVERT),
                    ActionInput::Verb {
                        target: *id,
                        verb: rs13_classic::verbs::REV_CONVERT.to_owned(),
                    },
                ));
            }
            for verb in rs13_classic::verbs::verbs_for(&appearance.sprite) {
                choices.push((
                    format!("verb-{verb}"),
                    ActionInput::Verb {
                        target: *id,
                        verb: verb.to_owned(),
                    },
                ));
            }
        }
        for (label, action) in choices {
            index += 1;
            choice(&mut commands, row, text(&label), MenuChoice { index, action });
        }
    }
    if let Some(turf) = turf {
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(menu),
            ))
            .id();
        commands.spawn((
            Text::new(noun_label(&turf)),
            ui_text_font(13.0),
            TextColor(Color::BLACK),
            Node {
                min_width: Val::Px(120.0),
                ..default()
            },
            ChildOf(row),
        ));
        for (label, action) in [
            ("menu-examine", ActionInput::ExamineTile(tile_pos)),
            ("menu-use", ActionInput::UseOnTile(tile_pos)),
        ] {
            index += 1;
            choice(&mut commands, row, text(label), MenuChoice { index, action });
        }
    }
}

fn choice(commands: &mut Commands, parent: Entity, label: String, choice: MenuChoice) {
    commands
        .spawn((
            choice,
            Node {
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.9, 0.9, 0.88)),
            BorderColor::all(EDGE),
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(12.0),
            TextColor(Color::srgb(0.0, 0.0, 0.6)),
            Pickable::IGNORE,
        ))
        .observe(click_choice);
}

fn click_choice(
    click: On<PointerClick>,
    choices: Query<&MenuChoice>,
    menus: Query<Entity, With<ContextMenu>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    mut commands: Commands,
) {
    let Ok(choice) = choices.get(click.entity) else {
        return;
    };
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(choice.action.clone());
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
}

/// A scripted `pick:N`, Escape, or a left click off the menu closes it (the click does
/// nothing else: `send_clicks` skips it).
pub fn close_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut request: ResMut<MenuRequest>,
    choices: Query<&MenuChoice>,
    menus: Query<(Entity, &Hovered), With<ContextMenu>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let picked = request.pick.take();
    if let Some(n) = picked
        && let Some(choice) = choices.iter().find(|c| c.index == n)
        && let Ok(mut sender) = sender.single_mut()
    {
        sender.send::<IntentChannel>(choice.action.clone());
    }
    let off_menu = mouse.just_pressed(MouseButton::Left) && menus.iter().all(|(_, h)| !h.0);
    if picked.is_some() || off_menu || keys.just_pressed(KeyCode::Escape) {
        for (menu, _) in &menus {
            commands.entity(menu).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn something_hung_over_shows_on_the_tile_it_is_drawn_on() {
        let at = TilePos::new(5, 5, 0);
        assert_eq!(drawn_tile(at, None), (5, 5));
        assert_eq!(drawn_tile(at, Some(&PixelOffset([24, 0]))), (6, 5));
        assert_eq!(drawn_tile(at, Some(&PixelOffset([0, -32]))), (5, 4));
        assert_eq!(drawn_tile(at, Some(&PixelOffset([8, -8]))), (5, 5), "only nudged");
    }
}

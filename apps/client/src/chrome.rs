//! Classic desktop windows: every window the client opens gets a raised grey frame and a navy
//! title bar carrying its title and a close box. Drag the title bar to move the window; it opens
//! where you left it next time. A press anywhere on a window brings it to the front. Buttons in
//! windows are raised grey keys that sink while pressed, and the right-click menu is a grey
//! popup. Panels build their windows as before: this dresses any unparented UI node at a window's
//! depth (`GlobalZIndex` 9 to 12, the menu at 20) the frame it appears.

use std::collections::HashMap;

use bevy::picking::events::{PointerClick, PointerDrag, PointerDragStart, PointerPress, PointerRelease};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use crate::{Loc, ui_text_font};

/// The face of windows and buttons.
pub const FACE: Color = Color::srgb_u8(192, 192, 192);
/// The lit top and left edges of something raised.
pub const LIGHT: Color = Color::srgb_u8(255, 255, 255);
/// The shaded bottom and right edges.
pub const SHADE: Color = Color::srgb_u8(64, 64, 64);
const TITLE_FROM: Color = Color::srgb_u8(0, 0, 128);
const TITLE_TO: Color = Color::srgb_u8(16, 132, 208);
const TITLE_TEXT: Color = Color::srgb_u8(255, 255, 255);
const KEY_TEXT: Color = Color::srgb_u8(0, 0, 0);
/// The depth a pressed window comes up to, below the right-click menu.
const FRONT_Z: i32 = 15;
/// How far each new window opens down and right of the last, logical pixels.
const CASCADE: f32 = 28.0;

/// A dressed window: what it's remembered by, and the depth its panel gave it.
#[derive(Component)]
pub struct Chrome {
    key: String,
    base_z: i32,
}

/// A window's title bar, and the window it moves.
#[derive(Component)]
struct TitleBar(Entity);

/// Where the window's top left was when the drag began, logical pixels.
#[derive(Component)]
struct DragFrom(Vec2);

/// A raised key: sinks while pressed.
#[derive(Component)]
struct Key;

/// The close box, and the panel's own Close button it presses.
#[derive(Component)]
struct CloseBox(Entity);

/// Where each window was last dragged to, by its panel.
#[derive(Resource, Default)]
pub struct WindowPlaces(HashMap<String, Vec2>);

fn raised() -> BorderColor {
    BorderColor {
        top: LIGHT,
        left: LIGHT,
        bottom: SHADE,
        right: SHADE,
    }
}

/// The edges of something set into the face: a text field, the log.
pub fn sunken() -> BorderColor {
    BorderColor {
        top: SHADE,
        left: SHADE,
        bottom: LIGHT,
        right: LIGHT,
    }
}

/// Dress the windows and the menu that appeared since last frame.
pub fn dress_windows(world: &mut World) {
    let mut roots =
        world.query_filtered::<(Entity, &GlobalZIndex), (With<Node>, Without<ChildOf>, Without<Chrome>)>();
    let new: Vec<(Entity, i32)> = roots
        .iter(world)
        .filter(|(_, z)| (9..=12).contains(&z.0) || z.0 == 20)
        .map(|(e, z)| (e, z.0))
        .collect();
    for (root, z) in new {
        let key = panel_key(world, root);
        world.entity_mut(root).insert(Chrome {
            key: key.clone(),
            base_z: z,
        });
        frame(world, root);
        if z == 20 {
            keys(world, root);
            continue;
        }
        place(world, root, &key);
        title_bar(world, root);
        keys(world, root);
    }
}

/// The panel's own marker (`pick_panel::PickWindow`), so a rebuilt window opens where it was.
fn panel_key(world: &World, root: Entity) -> String {
    let Ok(entity) = world.get_entity(root) else {
        return String::new();
    };
    entity
        .archetype()
        .iter_components()
        .filter_map(|id| world.components().get_info(id))
        .map(|info| info.name().to_string())
        .find(|name| name.starts_with("rs13_client::") && !name.contains("chrome"))
        .unwrap_or_default()
}

fn frame(world: &mut World, root: Entity) {
    let mut e = world.entity_mut(root);
    if let Some(mut node) = e.get_mut::<Node>() {
        node.border = UiRect::all(Val::Px(2.0));
    }
    e.insert((BackgroundColor(FACE), raised()));
}

/// Where it was last dragged to; or, never dragged, a step down and right for each other window
/// open, so a new one doesn't land exactly on another.
fn place(world: &mut World, root: Entity, key: &str) {
    let Some(at) = world.resource::<WindowPlaces>().0.get(key).copied() else {
        let mut open = world.query::<&Chrome>();
        let others = open
            .iter(world)
            .filter(|c| c.base_z != 20 && c.key != key)
            .count();
        let step = Val::Px(CASCADE * others as f32);
        if let Some(mut node) = world.get_mut::<Node>(root) {
            node.margin.left = step;
            node.margin.top = step;
        }
        return;
    };
    if let Some(mut node) = world.get_mut::<Node>(root) {
        node.left = Val::Px(at.x);
        node.top = Val::Px(at.y);
        node.right = Val::Auto;
        node.bottom = Val::Auto;
        node.margin = UiRect::ZERO;
    }
}

fn px(v: Val) -> f32 {
    match v {
        Val::Px(p) => p,
        _ => 0.0,
    }
}

/// A navy bar across the top, with the panel's title (its first line of text) and a close box.
fn title_bar(world: &mut World, root: Entity) {
    let pad = world.get::<Node>(root).map(|n| n.padding).unwrap_or_default();
    let first = world
        .get::<Children>(root)
        .and_then(|c| c.first().copied())
        .filter(|c| world.get::<Text>(*c).is_some());
    let close_label = world
        .resource::<Loc>()
        .0
        .format("panel-close", None)
        .unwrap_or_default();
    let close = find_button(world, root, &close_label);
    let bar = world
        .spawn((
            TitleBar(root),
            Node {
                flex_direction: FlexDirection::Row,
                align_self: AlignSelf::Stretch,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(12.0),
                padding: UiRect::new(Val::Px(4.0), Val::Px(2.0), Val::Px(2.0), Val::Px(2.0)),
                margin: UiRect::new(
                    Val::Px(-px(pad.left)),
                    Val::Px(-px(pad.right)),
                    Val::Px(-px(pad.top)),
                    Val::Px(4.0),
                ),
                min_height: Val::Px(20.0),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_right(vec![
                ColorStop::auto(TITLE_FROM),
                ColorStop::auto(TITLE_TO),
            ])),
        ))
        .observe(start_drag)
        .observe(drag)
        .id();
    world.entity_mut(root).insert_children(0, &[bar]);
    let text = match first {
        Some(text) => text,
        // A panel without a title line: its name from the locale (`window-vote-panel`).
        None => {
            let id = window_message(world.get::<Chrome>(root).map_or("", |c| c.key.as_str()));
            let title = world.resource::<Loc>().0.format(&id, None).unwrap_or_default();
            world.spawn((Text::new(title), ui_text_font(13.0))).id()
        }
    };
    let font = world
        .get::<TextFont>(text)
        .cloned()
        .unwrap_or_else(|| ui_text_font(13.0))
        .with_font_weight(FontWeight::BOLD);
    world
        .entity_mut(text)
        .insert((ChildOf(bar), TextColor(TITLE_TEXT), font, Pickable::IGNORE));
    if let Some(close) = close {
        world
            .spawn((
                CloseBox(close),
                Key,
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(16.0),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(FACE),
                raised(),
                ChildOf(bar),
            ))
            .with_child((
                Text::new("x"),
                ui_text_font(12.0),
                TextColor(KEY_TEXT),
                Pickable::IGNORE,
            ))
            .observe(press_key)
            .observe(release_key)
            .observe(close_window);
    }
    world.entity_mut(root).observe(to_front(root));
}

/// The locale message naming a panel's window: `rs13_client::vote_panel::VoteWindow` is
/// `window-vote-panel`.
fn window_message(key: &str) -> String {
    let module = key.split("::").nth(1).unwrap_or_default();
    format!("window-{}", module.replace('_', "-"))
}

/// The window's button labelled `label` (its Close), if it has one.
fn find_button(world: &World, root: Entity, label: &str) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if e != root
            && world.get::<Hovered>(e).is_some()
            && world.get::<Children>(e).is_some_and(|c| {
                c.iter()
                    .any(|t| world.get::<Text>(t).is_some_and(|t| t.0 == label))
            })
        {
            return Some(e);
        }
        if let Some(children) = world.get::<Children>(e) {
            stack.extend(children.iter());
        }
    }
    None
}

/// The window's buttons (what it lists to click) become raised grey keys with black labels.
fn keys(world: &mut World, root: Entity) {
    let mut stack = vec![root];
    let mut found = Vec::new();
    while let Some(e) = stack.pop() {
        if e != root
            && world.get::<Hovered>(e).is_some()
            && world.get::<TitleBar>(e).is_none()
            && world.get::<Key>(e).is_none()
        {
            found.push(e);
        }
        if let Some(children) = world.get::<Children>(e) {
            stack.extend(children.iter());
        }
    }
    for key in found {
        let labels: Vec<Entity> = world
            .get::<Children>(key)
            .map(|c| c.iter().filter(|t| world.get::<Text>(*t).is_some()).collect())
            .unwrap_or_default();
        for label in labels {
            world.entity_mut(label).insert(TextColor(KEY_TEXT));
        }
        let mut e = world.entity_mut(key);
        if let Some(mut node) = e.get_mut::<Node>() {
            node.border = UiRect::all(Val::Px(2.0));
            node.padding = UiRect::axes(Val::Px(8.0), Val::Px(2.0));
        }
        e.insert((Key, BackgroundColor(FACE), raised()));
        e.observe(press_key).observe(release_key);
    }
}

fn press_key(press: On<PointerPress>, mut keys: Query<&mut BorderColor, With<Key>>) {
    if let Ok(mut edge) = keys.get_mut(press.entity) {
        *edge = sunken();
    }
}

fn release_key(release: On<PointerRelease>, mut keys: Query<&mut BorderColor, With<Key>>) {
    if let Ok(mut edge) = keys.get_mut(release.entity) {
        *edge = raised();
    }
}

/// The close box presses the panel's own Close button, so the panel closes as it always has.
fn close_window(click: On<PointerClick>, boxes: Query<&CloseBox>, mut commands: Commands) {
    if let Ok(CloseBox(close)) = boxes.get(click.entity) {
        let mut pressed = click.event().clone();
        pressed.entity = *close;
        commands.trigger(pressed);
    }
}

fn start_drag(
    start: On<PointerDragStart>,
    bars: Query<&TitleBar>,
    windows: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut commands: Commands,
) {
    let Ok(TitleBar(window)) = bars.get(start.entity) else {
        return;
    };
    let Ok((computed, transform)) = windows.get(*window) else {
        return;
    };
    let centre = transform.to_scale_angle_translation().2;
    let top_left = (centre - computed.size() / 2.0) * computed.inverse_scale_factor;
    commands.entity(*window).insert(DragFrom(top_left));
}

fn drag(
    moved: On<PointerDrag>,
    bars: Query<&TitleBar>,
    mut windows: Query<(&mut Node, &DragFrom, &Chrome)>,
    mut places: ResMut<WindowPlaces>,
) {
    let Ok(TitleBar(window)) = bars.get(moved.entity) else {
        return;
    };
    let Ok((mut node, from, chrome)) = windows.get_mut(*window) else {
        return;
    };
    // Never so high that the title bar is lost off the top.
    let at = Vec2::new(
        from.0.x + moved.distance.x,
        (from.0.y + moved.distance.y).max(0.0),
    );
    node.left = Val::Px(at.x);
    node.top = Val::Px(at.y);
    node.right = Val::Auto;
    node.bottom = Val::Auto;
    // Its cascade step is part of where it now is.
    node.margin = UiRect::ZERO;
    places.0.insert(chrome.key.clone(), at);
}

/// A press on a window brings it in front of the others.
fn to_front(window: Entity) -> impl Fn(On<PointerPress>, Query<(Entity, &Chrome, &mut GlobalZIndex)>) {
    move |_press, mut windows| {
        for (e, chrome, mut z) in &mut windows {
            if chrome.base_z == 20 {
                continue;
            }
            let want = if e == window { FRONT_Z } else { chrome.base_z };
            if z.0 != want {
                z.0 = want;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panel_window_is_named_from_its_module() {
        assert_eq!(
            window_message("rs13_client::vote_panel::VoteWindow"),
            "window-vote-panel"
        );
        assert_eq!(window_message(""), "window-");
    }
}

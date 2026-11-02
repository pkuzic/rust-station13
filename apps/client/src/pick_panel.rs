//! A window listing things to pick one from, such as a filing cabinet's sheets (Luna's
//! `filingcabinet` window): its name as the title, each thing a link, and Close.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, PickListPanel};
use rs13_world::PickListView;

use crate::{Loc, ui_text_font};

/// The list the server says is open.
#[derive(Resource, Default)]
pub struct OpenPickList(pub Option<PickListView>);

#[derive(Component)]
pub struct PickWindow;

#[derive(Component, Clone, Copy)]
pub enum PickButton {
    Line(u16),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_pick_list(
    mut open: ResMut<OpenPickList>,
    mut receivers: Query<&mut MessageReceiver<PickListPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(PickListPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_pick_list(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenPickList>,
    windows: Query<Entity, With<PickWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let title = loc
        .0
        .format_msg(&LocMsg::new("pick-list-title").arg("item", view.title.clone()));

    let window = commands
        .spawn((
            PickWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    commands.spawn((
        Text::new(title),
        ui_text_font(14.0),
        TextColor(TEXT),
        ChildOf(window),
    ));
    for (i, row) in view.rows.iter().enumerate() {
        button(
            &mut commands,
            window,
            row.clone(),
            PickButton::Line(i as u16),
            LINK,
        );
    }
    let close = loc.0.format("panel-close", None).unwrap_or_default();
    button(&mut commands, window, close, PickButton::Close, TEXT);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: PickButton, colour: Color) {
    commands
        .spawn((
            action,
            Node {
                padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
                ..default()
            },
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(13.0),
            TextColor(colour),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&PickButton>,
    mut open: ResMut<OpenPickList>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        PickButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Pick {
                    target: view.target,
                    line,
                });
            }
        }
        PickButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

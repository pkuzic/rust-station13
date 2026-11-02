//! The space heater's window (Luna's `spaceheater` window, open through its hatch): the
//! cell (a link to take it out or put one in), its charge, and the temperature it heats to
//! with - and +.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, HeaterCommand, HeaterPanel, IntentChannel};
use rs13_world::HeaterPanelView;

use crate::{Loc, ui_text_font};

/// The heater the server says is open.
#[derive(Resource, Default)]
pub struct OpenHeater(pub Option<HeaterPanelView>);

#[derive(Component)]
pub struct HeaterWindow;

#[derive(Component, Clone, Copy)]
pub enum HeaterButton {
    Command(HeaterCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_heater(
    mut open: ResMut<OpenHeater>,
    mut receivers: Query<&mut MessageReceiver<HeaterPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(HeaterPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_heater(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenHeater>,
    windows: Query<Entity, With<HeaterWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);

    let window = commands
        .spawn((
            HeaterWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
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
    let row = |commands: &mut Commands| {
        commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id()
    };
    let label = |commands: &mut Commands, parent: Entity, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(parent)));
    };
    label(&mut commands, window, text(LocMsg::new("heater-title")));
    let cell = row(&mut commands);
    label(&mut commands, cell, text(LocMsg::new("heater-cell")));
    let state = if view.has_cell {
        "heater-cell-installed"
    } else {
        "heater-cell-removed"
    };
    button(
        &mut commands,
        cell,
        text(LocMsg::new(state)),
        HeaterButton::Command(HeaterCommand::Cell),
    );
    label(
        &mut commands,
        window,
        text(LocMsg::new("heater-level").num("value", i64::from(view.charge_percent))),
    );
    let setting = row(&mut commands);
    label(&mut commands, setting, text(LocMsg::new("heater-set")));
    button(
        &mut commands,
        setting,
        "-".to_owned(),
        HeaterButton::Command(HeaterCommand::Warmer(false)),
    );
    label(
        &mut commands,
        setting,
        text(LocMsg::new("heater-degrees").num("value", i64::from(view.setting))),
    );
    button(
        &mut commands,
        setting,
        "+".to_owned(),
        HeaterButton::Command(HeaterCommand::Warmer(true)),
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        HeaterButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: HeaterButton) {
    commands
        .spawn((
            action,
            Node {
                padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                ..default()
            },
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(14.0),
            TextColor(LINK),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&HeaterButton>,
    mut open: ResMut<OpenHeater>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        HeaterButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Heater {
                    target: view.target,
                    command,
                });
            }
        }
        HeaterButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

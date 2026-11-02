//! The security camera console's window (Luna's camera list): each camera by its tag,
//! "(Deactivated)" when off, a button to watch it, Cancel to stop, and Close.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, CameraPanel, IntentChannel};
use rs13_world::CameraPanelView;

use crate::{Loc, ui_text_font};

/// The console the server says is open.
#[derive(Resource, Default)]
pub struct OpenCameras(pub Option<CameraPanelView>);

#[derive(Component)]
pub struct CameraWindow;

#[derive(Component, Clone, Copy)]
pub enum CameraButton {
    Watch(u16),
    Cancel,
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const OFF: Color = Color::srgb(0.5, 0.5, 0.5);

pub fn receive_cameras(
    mut open: ResMut<OpenCameras>,
    mut receivers: Query<&mut MessageReceiver<CameraPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(CameraPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_cameras(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenCameras>,
    windows: Query<Entity, With<CameraWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();

    let window = commands
        .spawn((
            CameraWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(320.0),
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
    let line = |commands: &mut Commands, s: String, colour: Color, parent: Entity, width: f32| {
        commands.spawn((
            Text::new(s),
            ui_text_font(14.0),
            TextColor(colour),
            Node {
                min_width: Val::Px(width),
                ..default()
            },
            ChildOf(parent),
        ));
    };
    line(&mut commands, text("cameras-title"), TEXT, window, 0.0);
    line(&mut commands, text("cameras-select"), TEXT, window, 0.0);
    for (i, row) in view.cameras.iter().enumerate() {
        let entry = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id();
        let watching = view.current == Some(i as u16);
        let mut label = row.tag.clone();
        if !row.on {
            label.push_str(&text("cameras-deactivated"));
        }
        if watching {
            label = format!("> {label}");
        }
        line(
            &mut commands,
            label,
            if row.on { TEXT } else { OFF },
            entry,
            220.0,
        );
        if row.on && !watching {
            button(
                &mut commands,
                entry,
                text("cameras-view"),
                CameraButton::Watch(i as u16),
            );
        }
    }
    let close = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    button(&mut commands, close, text("cameras-cancel"), CameraButton::Cancel);
    button(&mut commands, close, text("panel-close"), CameraButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: CameraButton) {
    commands
        .spawn((
            action,
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
            ui_text_font(13.0),
            TextColor(Color::srgb(0.0, 0.0, 0.6)),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&CameraButton>,
    mut open: ResMut<OpenCameras>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        CameraButton::Watch(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Camera {
                    target: view.target,
                    line: Some(line),
                });
            }
        }
        CameraButton::Cancel => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Camera {
                    target: view.target,
                    line: None,
                });
            }
        }
        CameraButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

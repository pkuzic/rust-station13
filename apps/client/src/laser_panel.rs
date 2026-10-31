//! The laser control computer's window (Luna's `lascomp` browser window): the lasers'
//! power level with its eight buttons, and whether they fire.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, LaserCommand, LaserPanel};
use rs13_world::LaserPanelView;

use crate::{Loc, ui_text_font};

/// The laser computer the server says is open.
#[derive(Resource, Default)]
pub struct OpenLaser(pub Option<LaserPanelView>);

#[derive(Component)]
pub struct LaserWindow;

#[derive(Component, Clone, Copy)]
pub enum LaserButton {
    Command(LaserCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
/// Luna's power buttons: ±1000, ±100, ±10, ±1.
const STEPS: [i32; 4] = [1000, 100, 10, 1];

pub fn receive_laser(
    mut open: ResMut<OpenLaser>,
    mut receivers: Query<&mut MessageReceiver<LaserPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(LaserPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_laser(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenLaser>,
    windows: Query<Entity, With<LaserWindow>>,
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
            LaserWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(5.0),
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
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id()
    };
    let label = |commands: &mut Commands, parent: Entity, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(parent)));
    };

    let title = row(&mut commands);
    label(&mut commands, title, text("laser-title"));
    if view.lasers == 0 {
        let none = row(&mut commands);
        label(&mut commands, none, text("laser-none"));
    } else {
        let power = row(&mut commands);
        label(&mut commands, power, text("laser-power"));
        for step in STEPS {
            button(
                &mut commands,
                power,
                "-".into(),
                LaserButton::Command(LaserCommand::Power(-step)),
            );
        }
        label(&mut commands, power, view.power.to_string());
        for step in STEPS.into_iter().rev() {
            button(
                &mut commands,
                power,
                "+".into(),
                LaserButton::Command(LaserCommand::Power(step)),
            );
        }
        let output = row(&mut commands);
        let mut args = rs13_i18n::Args::new();
        args.set("on", if view.online { "yes" } else { "no" });
        label(
            &mut commands,
            output,
            loc.0.format("laser-output", Some(&args)).unwrap_or_default(),
        );
        button(
            &mut commands,
            output,
            text("apc-toggle"),
            LaserButton::Command(LaserCommand::Toggle),
        );
    }
    let close = row(&mut commands);
    button(&mut commands, close, text("panel-close"), LaserButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: LaserButton) {
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
    buttons: Query<&LaserButton>,
    mut open: ResMut<OpenLaser>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        LaserButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Laser {
                    target: view.target,
                    command,
                });
            }
        }
        LaserButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

//! The air alarm window (Luna's "alarm" browser window): the room's readouts in blue, or
//! red when out of range, then the seal and panic fill buttons when in reach, and Close.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, AlarmCommand, AlarmPanel, IntentChannel};
use rs13_world::AlarmPanelView;

use crate::{Loc, ui_text_font};

/// The alarm window the server says is open.
#[derive(Resource, Default)]
pub struct OpenAlarm(pub Option<AlarmPanelView>);

#[derive(Component)]
pub struct AlarmWindow;

#[derive(Component, Clone, Copy)]
pub enum AlarmButton {
    Command(AlarmCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const NORMAL: Color = Color::srgb(0.0, 0.0, 0.8);
const ALERT: Color = Color::srgb(0.8, 0.0, 0.0);

pub fn receive_alarm(
    mut open: ResMut<OpenAlarm>,
    mut receivers: Query<&mut MessageReceiver<AlarmPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(AlarmPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever the readouts change.
pub fn show_alarm(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenAlarm>,
    windows: Query<Entity, With<AlarmWindow>>,
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
            AlarmWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    let line = |commands: &mut Commands, s: String, colour: Color| {
        commands.spawn((
            Text::new(s),
            ui_text_font(14.0),
            TextColor(colour),
            ChildOf(window),
        ));
    };

    let mut args = rs13_i18n::Args::new();
    args.set("area", text(&format!("area-{}", view.area)));
    line(
        &mut commands,
        loc.0.format("alarm-title", Some(&args)).unwrap_or_default(),
        NORMAL,
    );
    let mut args = rs13_i18n::Args::new();
    args.set("value", f64::from(view.pressure_dkpa) / 10.0);
    line(
        &mut commands,
        loc.0.format("alarm-pressure", Some(&args)).unwrap_or_default(),
        if view.pressure_alert { ALERT } else { NORMAL },
    );
    for reading in &view.readings {
        let mut args = rs13_i18n::Args::new();
        args.set("value", reading.value);
        line(
            &mut commands,
            loc.0.format(&reading.id, Some(&args)).unwrap_or_default(),
            if reading.alert { ALERT } else { NORMAL },
        );
    }

    let buttons = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                row_gap: Val::Px(4.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    if view.controls {
        let (label, command) = if view.sealed {
            ("alarm-seal-off", AlarmCommand::Seal(false))
        } else {
            ("alarm-seal-on", AlarmCommand::Seal(true))
        };
        button(&mut commands, buttons, text(label), AlarmButton::Command(command));
        match view.panic_fill {
            Some(on) => {
                let (label, command) = if on {
                    ("alarm-panic-off", AlarmCommand::PanicFill(false))
                } else {
                    ("alarm-panic-on", AlarmCommand::PanicFill(true))
                };
                button(&mut commands, buttons, text(label), AlarmButton::Command(command));
            }
            None => {
                commands.spawn((
                    Text::new(text("alarm-no-vents")),
                    ui_text_font(14.0),
                    TextColor(ALERT),
                    ChildOf(buttons),
                ));
            }
        }
    }
    button(&mut commands, buttons, text("panel-close"), AlarmButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: AlarmButton) {
    commands
        .spawn((
            action,
            Node {
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
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
    buttons: Query<&AlarmButton>,
    mut open: ResMut<OpenAlarm>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        AlarmButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Alarm {
                    target: view.target,
                    command,
                });
            }
        }
        AlarmButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

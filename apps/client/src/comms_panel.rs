//! A communications console's window (Luna's `comm_console`): call the escape pods, or stop them.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, CommsCommand, CommsPanel, IntentChannel};
use rs13_world::CommsPanelView;

use crate::{Loc, ui_text_font};

/// The console the server says is open.
#[derive(Resource, Default)]
pub struct OpenComms(pub Option<CommsPanelView>);

#[derive(Component)]
pub struct CommsWindow;

#[derive(Clone, Copy)]
pub enum Sent {
    Call,
    Stop,
    RedAlert,
    DisplayPods,
    DisplayBlank,
    DisplayPicture(u8),
}

impl Sent {
    fn command(self) -> CommsCommand {
        match self {
            Sent::Call => CommsCommand::Call,
            Sent::Stop => CommsCommand::Stop,
            Sent::RedAlert => CommsCommand::RedAlert,
            Sent::DisplayPods => CommsCommand::DisplayPods,
            Sent::DisplayBlank => CommsCommand::DisplayBlank,
            Sent::DisplayPicture(line) => CommsCommand::DisplayPicture(line),
        }
    }
}

#[derive(Component, Clone, Copy)]
pub enum CommsButton {
    Send(Sent),
    /// Open the chat line for a display message.
    Message,
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_comms(
    mut open: ResMut<OpenComms>,
    mut receivers: Query<&mut MessageReceiver<CommsPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(CommsPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_comms(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenComms>,
    windows: Query<Entity, With<CommsWindow>>,
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
            CommsWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(320.0),
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
    let label = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(window)));
    };
    label(&mut commands, text(LocMsg::new("comms-title")));
    match (view.launch_in, view.gone) {
        (_, true) => label(&mut commands, text(LocMsg::new("comms-gone"))),
        (Some(seconds), _) => {
            label(
                &mut commands,
                text(LocMsg::new("comms-launching").num("value", i64::from(seconds))),
            );
            button(
                &mut commands,
                window,
                text(LocMsg::new("comms-recall")),
                CommsButton::Send(Sent::Stop),
            );
        }
        (None, _) => button(
            &mut commands,
            window,
            text(LocMsg::new("comms-call")),
            CommsButton::Send(Sent::Call),
        ),
    }
    let red = if view.red_alert {
        "comms-red-off"
    } else {
        "comms-red-on"
    };
    button(
        &mut commands,
        window,
        text(LocMsg::new(red)),
        CommsButton::Send(Sent::RedAlert),
    );
    label(&mut commands, text(LocMsg::new("comms-display-title")));
    button(
        &mut commands,
        window,
        text(LocMsg::new("comms-display-pods")),
        CommsButton::Send(Sent::DisplayPods),
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("comms-display-blank")),
        CommsButton::Send(Sent::DisplayBlank),
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("comms-display-message")),
        CommsButton::Message,
    );
    for (line, id) in ["default", "redalert", "lockdown", "biohazard"]
        .iter()
        .enumerate()
    {
        button(
            &mut commands,
            window,
            text(LocMsg::new(format!("comms-picture-{id}"))),
            CommsButton::Send(Sent::DisplayPicture(line as u8)),
        );
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        CommsButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: CommsButton) {
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
    buttons: Query<&CommsButton>,
    mut open: ResMut<OpenComms>,
    mut chat: ResMut<crate::ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        CommsButton::Send(sent) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Comms {
                    target: view.target,
                    command: sent.command(),
                });
            }
        }
        CommsButton::Message => {
            if let Some(view) = &open.0 {
                chat.open = true;
                chat.buffer.clear();
                chat.kind = crate::ChatKind::Display;
                chat.console = Some(view.target);
            }
        }
        CommsButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

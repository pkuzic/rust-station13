//! The window of the nuclear bomb (Luna's `nuclearbomb`). Spec: `docs/reference/nuke.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, NukeCommand, NukePanel};
use rs13_world::NukePanelView;

use crate::{Loc, ui_text_font};

/// The bomb the server says is open.
#[derive(Resource, Default)]
pub struct OpenNuke(pub Option<NukePanelView>);

#[derive(Component)]
pub struct NukeWindow;

#[derive(Component, Clone, Copy)]
pub enum NukeButton {
    Do(NukeCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_nuke(
    mut open: ResMut<OpenNuke>,
    mut receivers: Query<&mut MessageReceiver<NukePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(NukePanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_nuke(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenNuke>,
    windows: Query<Entity, With<NukeWindow>>,
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
            NukeWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(300.0),
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
    let onoff = |on: bool| if on { "nuke-on" } else { "nuke-off" };
    label(&mut commands, text(LocMsg::new("nuke-title")));
    button(
        &mut commands,
        window,
        text(LocMsg::new(if view.disk {
            "nuke-disk-in"
        } else {
            "nuke-disk-out"
        })),
        NukeButton::Do(NukeCommand::Disk),
    );
    label(
        &mut commands,
        text(LocMsg::new("nuke-time").num("value", i64::from(view.time_left))),
    );
    let lines = [
        ("nuke-timer", view.timing, NukeCommand::Timer),
        ("nuke-safety", view.safety, NukeCommand::Safety),
        ("nuke-anchor", view.anchored, NukeCommand::Anchor),
    ];
    for (id, on, command) in lines {
        let shown = text(LocMsg::new(id).msg("state", onoff(on)));
        if view.code_ok {
            button(&mut commands, window, shown, NukeButton::Do(command));
        } else {
            label(&mut commands, shown);
        }
    }
    if view.code_ok {
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(10.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id();
        for change in [-10i8, -1, 1, 10] {
            button(
                &mut commands,
                row,
                format!("{change:+}"),
                NukeButton::Do(NukeCommand::Time(change)),
            );
        }
    }
    let shown = if !view.disk {
        text(LocMsg::new("nuke-display-auth"))
    } else if view.code_ok {
        "*****".to_owned()
    } else {
        view.display.clone()
    };
    label(&mut commands, format!("> {shown}"));
    let keys = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                column_gap: Val::Px(12.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for digit in 1..=9u8 {
        button(
            &mut commands,
            keys,
            digit.to_string(),
            NukeButton::Do(NukeCommand::Key(digit)),
        );
    }
    button(
        &mut commands,
        keys,
        "R".to_owned(),
        NukeButton::Do(NukeCommand::Reset),
    );
    button(
        &mut commands,
        keys,
        "0".to_owned(),
        NukeButton::Do(NukeCommand::Key(0)),
    );
    button(
        &mut commands,
        keys,
        "E".to_owned(),
        NukeButton::Do(NukeCommand::Enter),
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        NukeButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: NukeButton) {
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
    buttons: Query<&NukeButton>,
    mut open: ResMut<OpenNuke>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        NukeButton::Do(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Nuke {
                    target: view.target,
                    command,
                });
            }
        }
        NukeButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

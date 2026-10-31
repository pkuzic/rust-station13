//! The APC window (Luna's "apc" browser window): breaker, grid power, cell, the three
//! channels and the cover lock. Locked, it only shows them; unlocked and in reach, each
//! line has its buttons.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ApcChannelSetting, ApcCommand, ApcPanel, IntentChannel};
use rs13_world::ApcPanelView;

use crate::{Loc, ui_text_font};

/// The APC window the server says is open.
#[derive(Resource, Default)]
pub struct OpenApc(pub Option<ApcPanelView>);

#[derive(Component)]
pub struct ApcWindow;

#[derive(Component, Clone, Copy)]
pub enum ApcButton {
    Command(ApcCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const HINT: Color = Color::srgb(0.3, 0.3, 0.3);

/// Channel message ids, in the view's order.
const CHANNELS: [&str; 3] = ["apc-equipment", "apc-lighting", "apc-environment"];

pub fn receive_apc(
    mut open: ResMut<OpenApc>,
    mut receivers: Query<&mut MessageReceiver<ApcPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ApcPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_apc(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenApc>,
    windows: Query<Entity, With<ApcWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let say = |id: &str, fill: &dyn Fn(&mut rs13_i18n::Args)| {
        let mut a = rs13_i18n::Args::new();
        fill(&mut a);
        loc.0.format(id, Some(&a)).unwrap_or_default()
    };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let buttons = !view.locked && view.controls;

    let window = commands
        .spawn((
            ApcWindow,
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
    // A line of text, then its buttons.
    let row = |commands: &mut Commands, s: String, colour: Color, choices: Vec<(String, ApcCommand)>| {
        let line = commands
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
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(colour), ChildOf(line)));
        for (label, command) in choices {
            button(commands, line, label, ApcButton::Command(command));
        }
    };

    let area = text(&format!("area-{}", view.area));
    row(
        &mut commands,
        say("apc-title", &|a| a.set("area", area.clone())),
        TEXT,
        Vec::new(),
    );
    let hint = if view.locked {
        "apc-hint-locked"
    } else {
        "apc-hint-unlocked"
    };
    row(&mut commands, text(hint), HINT, Vec::new());

    let toggle = |command: ApcCommand| {
        if buttons {
            vec![(text("apc-toggle"), command)]
        } else {
            Vec::new()
        }
    };
    let on = |b: bool| if b { "yes" } else { "no" };
    row(
        &mut commands,
        say("apc-breaker", &|a| a.set("on", on(view.breaker))),
        TEXT,
        toggle(ApcCommand::Breaker),
    );
    row(
        &mut commands,
        say("apc-external", &|a| a.set("state", i64::from(view.external))),
        TEXT,
        Vec::new(),
    );
    match view.cell_percent {
        Some(percent) => row(
            &mut commands,
            say("apc-cell", &|a| {
                a.set("value", i64::from(percent));
                a.set("state", i64::from(view.charging));
                a.set("auto", on(view.charge_auto));
            }),
            TEXT,
            toggle(ApcCommand::ChargeMode),
        ),
        None => row(&mut commands, text("apc-no-cell"), TEXT, Vec::new()),
    }
    for (i, (mode, watts)) in view.channels.iter().enumerate() {
        let choices = if buttons {
            [
                ("apc-set-off", ApcChannelSetting::Off),
                ("apc-set-on", ApcChannelSetting::On),
                ("apc-set-auto", ApcChannelSetting::Auto),
            ]
            .into_iter()
            .map(|(label, mode)| {
                let channel = i as u8;
                (text(label), ApcCommand::Channel { channel, mode })
            })
            .collect()
        } else {
            Vec::new()
        };
        row(
            &mut commands,
            say(CHANNELS[i], &|a| {
                a.set("load", i64::from(*watts));
                a.set("mode", i64::from(*mode));
            }),
            TEXT,
            choices,
        );
    }
    row(
        &mut commands,
        say("apc-cover-lock", &|a| a.set("on", on(view.cover_locked))),
        TEXT,
        toggle(ApcCommand::CoverLock),
    );
    let close = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    button(&mut commands, close, text("panel-close"), ApcButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: ApcButton) {
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
    buttons: Query<&ApcButton>,
    mut open: ResMut<OpenApc>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        ApcButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Apc {
                    target: view.target,
                    command,
                });
            }
        }
        ApcButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

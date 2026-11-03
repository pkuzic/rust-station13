//! A records computer's window (Luna's `secure_data` and `med_data` windows): who's aboard, what
//! they do, and their criminal status or condition.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, RecordsPanel};
use rs13_world::RecordsPanelView;

use crate::{Loc, ui_text_font};

/// The records computer the server says is open.
#[derive(Resource, Default)]
pub struct OpenRecords(pub Option<RecordsPanelView>);

#[derive(Component)]
pub struct RecordsWindow;

#[derive(Component, Clone, Copy)]
pub enum RecordsButton {
    Line(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_records(
    mut open: ResMut<OpenRecords>,
    mut receivers: Query<&mut MessageReceiver<RecordsPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(RecordsPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_records(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenRecords>,
    windows: Query<Entity, With<RecordsWindow>>,
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
            RecordsWindow,
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
    let title = if view.medical {
        "records-medical"
    } else {
        "records-security"
    };
    label(&mut commands, text(LocMsg::new(title)));
    if view.rows.is_empty() {
        label(&mut commands, text(LocMsg::new("records-empty")));
    }
    for (line, row) in view.rows.iter().enumerate() {
        let shown = text(
            LocMsg::new("records-row")
                .arg("name", row.name.clone())
                .arg("job", text(LocMsg::new(format!("job-{}", row.job))))
                .arg("status", text(LocMsg::new(format!("record-{}", row.status)))),
        );
        if view.medical {
            label(&mut commands, shown);
        } else {
            button(&mut commands, window, shown, RecordsButton::Line(line as u8));
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        RecordsButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: RecordsButton) {
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
    buttons: Query<&RecordsButton>,
    mut open: ResMut<OpenRecords>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        RecordsButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Records {
                    target: view.target,
                    line,
                });
            }
        }
        RecordsButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

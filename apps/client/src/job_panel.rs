//! The job window a new arrival sees: the job they've been given, and the others still open.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, JobPanel};
use rs13_world::JobPanelView;

use crate::{Loc, ui_text_font};

/// The job window the server says is open.
#[derive(Resource, Default)]
pub struct OpenJob(pub Option<JobPanelView>);

#[derive(Component)]
pub struct JobWindow;

#[derive(Component, Clone, Copy)]
pub enum JobButton {
    Line(u8),
    Stay,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);
const GREY: Color = Color::srgb(0.45, 0.45, 0.45);

pub fn receive_job(
    mut open: ResMut<OpenJob>,
    mut receivers: Query<&mut MessageReceiver<JobPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(JobPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_job(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenJob>,
    windows: Query<Entity, With<JobWindow>>,
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
            JobWindow,
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
    label(
        &mut commands,
        text(LocMsg::new("job-window-title").arg("job", view.current.clone())),
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("job-keep")),
        JobButton::Stay,
    );
    let columns = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for (line, (job, open)) in view.rows.iter().enumerate() {
        let name = text(LocMsg::new(format!("job-{job}")));
        if *open > 0 {
            let shown = text(
                LocMsg::new("job-row")
                    .arg("job", name)
                    .num("value", i64::from(*open)),
            );
            button(&mut commands, columns, shown, JobButton::Line(line as u8));
        } else {
            commands.spawn((
                Text::new(name),
                ui_text_font(14.0),
                TextColor(GREY),
                ChildOf(columns),
            ));
        }
    }
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: JobButton) {
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
    buttons: Query<&JobButton>,
    mut open: ResMut<OpenJob>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let line = match *action {
        JobButton::Line(line) => Some(line),
        JobButton::Stay => None,
    };
    sender.send::<IntentChannel>(ActionInput::ChooseJob { line });
    open.0 = None;
}

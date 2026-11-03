//! The lobby (Luna's `new_player` window): the countdown, ready or join, your name, and the job you'd
//! like. Spec: `docs/reference/round.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, LobbyPanel};
use rs13_world::LobbyView;

use crate::{Loc, ui_text_font};

/// The lobby the server says we are in.
#[derive(Resource, Default)]
pub struct OpenLobby(pub Option<LobbyView>);

#[derive(Component)]
pub struct LobbyWindow;

#[derive(Component, Clone, Copy)]
pub enum LobbyButton {
    Ready(bool),
    Prefer(u8),
    Join,
    Vote,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);
const GREY: Color = Color::srgb(0.45, 0.45, 0.45);

pub fn receive_lobby(
    mut open: ResMut<OpenLobby>,
    mut receivers: Query<&mut MessageReceiver<LobbyPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(LobbyPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_lobby(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenLobby>,
    windows: Query<Entity, With<LobbyWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    // Over everything: there's nothing to see until you're in the world.
    let backdrop = commands
        .spawn((
            LobbyWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Hovered::default(),
            GlobalZIndex(9),
        ))
        .id();
    let window = commands
        .spawn((
            Node {
                min_width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            ChildOf(backdrop),
        ))
        .id();
    let label = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(window)));
    };
    label(&mut commands, text(LocMsg::new("lobby-title")));
    match view.starts_in {
        Some(seconds) => {
            label(
                &mut commands,
                text(
                    LocMsg::new("lobby-countdown")
                        .num("value", i64::from(seconds))
                        .num("ready", i64::from(view.ready_count))
                        .num("players", i64::from(view.players)),
                ),
            );
            let (id, next) = if view.ready {
                ("lobby-unready", false)
            } else {
                ("lobby-ready", true)
            };
            button(
                &mut commands,
                window,
                text(LocMsg::new(id)),
                LobbyButton::Ready(next),
            );
        }
        None => {
            label(&mut commands, text(LocMsg::new("lobby-round-on")));
            button(
                &mut commands,
                window,
                text(LocMsg::new("lobby-join")),
                LobbyButton::Join,
            );
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("lobby-vote")),
        LobbyButton::Vote,
    );
    label(
        &mut commands,
        match &view.name {
            Some(name) => text(LocMsg::new("lobby-name").arg("name", name.clone())),
            None => text(LocMsg::new("lobby-no-name")),
        },
    );
    label(&mut commands, text(LocMsg::new("lobby-prefer")));
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
    for (line, (job, open)) in view.jobs.iter().enumerate() {
        let name = text(LocMsg::new(format!("job-{job}")));
        let chosen = view.preference.as_ref() == Some(job);
        if *open > 0 {
            let shown = text(
                LocMsg::new("job-row")
                    .arg("job", name)
                    .num("value", i64::from(*open)),
            );
            let shown = if chosen { format!("[{shown}]") } else { shown };
            button(&mut commands, columns, shown, LobbyButton::Prefer(line as u8));
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

fn button(commands: &mut Commands, parent: Entity, label: String, action: LobbyButton) {
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
    buttons: Query<&LobbyButton>,
    mut votes: ResMut<crate::vote_panel::OpenVote>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    if matches!(*action, LobbyButton::Vote) {
        votes.open = true;
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    sender.send::<IntentChannel>(match *action {
        LobbyButton::Ready(ready) => ActionInput::Ready(ready),
        LobbyButton::Prefer(line) => ActionInput::ChooseJob { line: Some(line) },
        LobbyButton::Join => ActionInput::JoinRound,
        LobbyButton::Vote => return,
    });
}

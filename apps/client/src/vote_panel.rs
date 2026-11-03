//! The vote window (Luna's `Vote` verb), opened with V or from the lobby. Spec:
//! `docs/reference/round.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, VoteAction, VotePanel};
use rs13_world::VoteView;

use crate::{ChatInput, Loc, ui_text_font};

/// How voting stands, and whether this player has the window open.
#[derive(Resource, Default)]
pub struct OpenVote {
    pub view: Option<VoteView>,
    pub open: bool,
}

#[derive(Component)]
pub struct VoteWindow;

#[derive(Component, Clone, Copy)]
pub enum VoteButton {
    Act(VoteAction),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_vote(
    mut open: ResMut<OpenVote>,
    mut receivers: Query<&mut MessageReceiver<VotePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(VotePanel(view)) = receiver.receive().last() {
            open.view = Some(view);
        }
    }
}

/// V opens and closes it, whenever the chat line isn't open.
pub fn toggle_vote(keys: Res<ButtonInput<KeyCode>>, chat: Res<ChatInput>, mut open: ResMut<OpenVote>) {
    if !chat.open && !chat.is_changed() && keys.just_pressed(KeyCode::KeyV) {
        open.open = !open.open;
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_vote(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenVote>,
    windows: Query<Entity, With<VoteWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let (Some(view), true) = (&open.view, open.open) else {
        return;
    };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let window = commands
        .spawn((
            VoteWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(1.0),
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
    match &view.kind {
        Some(kind) => {
            label(
                &mut commands,
                text(LocMsg::new("vote-title").arg("kind", kind.clone())),
            );
            label(
                &mut commands,
                text(LocMsg::new("vote-closes-in").num("value", i64::from(view.ends_in))),
            );
            for (line, (choice, count)) in view.choices.iter().enumerate() {
                let name = text(LocMsg::new(format!("vote-choice-{choice}")));
                let shown = if view.mine.as_ref() == Some(choice) {
                    format!("[{name}] - {count}")
                } else {
                    format!("{name} - {count}")
                };
                button(
                    &mut commands,
                    window,
                    shown,
                    VoteButton::Act(VoteAction::Cast(line as u8)),
                );
            }
            if let Some(leading) = &view.leading {
                let leading = LocMsg::new("vote-leading").msg("choice", format!("vote-choice-{leading}"));
                label(&mut commands, text(leading));
            }
        }
        None if view.next_in > 0 => {
            label(
                &mut commands,
                text(LocMsg::new("vote-next-in").num("value", i64::from(view.next_in))),
            );
        }
        None => {
            button(
                &mut commands,
                window,
                text(LocMsg::new("vote-begin-restart")),
                VoteButton::Act(VoteAction::BeginRestart),
            );
            if view.can_mode {
                button(
                    &mut commands,
                    window,
                    text(LocMsg::new("vote-begin-mode")),
                    VoteButton::Act(VoteAction::BeginMode),
                );
            }
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        VoteButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: VoteButton) {
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
    buttons: Query<&VoteButton>,
    mut open: ResMut<OpenVote>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    match *action {
        VoteButton::Act(action) => {
            if let Ok(mut sender) = sender.single_mut() {
                sender.send::<IntentChannel>(ActionInput::Vote(action));
            }
        }
        VoteButton::Close => open.open = false,
    }
}

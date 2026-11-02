//! An arcade machine's window (Luna's `arcade` window): the villain, what just happened, how
//! the fight stands, and Attack, Heal and Recharge Power, or New Game once it's over.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ArcadeCommand, ArcadePanel, IntentChannel};
use rs13_world::ArcadePanelView;

use crate::{Loc, ui_text_font};

/// The game the server says is open.
#[derive(Resource, Default)]
pub struct OpenArcade(pub Option<ArcadePanelView>);

#[derive(Component)]
pub struct ArcadeWindow;

#[derive(Component, Clone, Copy)]
pub enum ArcadeButton {
    Command(ArcadeCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_arcade(
    mut open: ResMut<OpenArcade>,
    mut receivers: Query<&mut MessageReceiver<ArcadePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ArcadePanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_arcade(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenArcade>,
    windows: Query<Entity, With<ArcadeWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let enemy = text(
        LocMsg::new("arcade-villain")
            .arg("title", view.title.clone())
            .arg("villain", view.villain.clone()),
    );

    let window = commands
        .spawn((
            ArcadeWindow,
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
    label(&mut commands, enemy.clone());
    label(
        &mut commands,
        text(
            LocMsg::new(view.message.clone())
                .arg("enemy", enemy)
                .num("value", i64::from(view.value))
                .num("cost", i64::from(view.cost)),
        ),
    );
    label(
        &mut commands,
        text(
            LocMsg::new("arcade-stats")
                .num("hp", i64::from(view.player_hp))
                .num("mp", i64::from(view.player_mp))
                .num("enemyhp", i64::from(view.enemy_hp)),
        ),
    );
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    if view.game_over {
        button(
            &mut commands,
            row,
            text(LocMsg::new("arcade-button-new")),
            ArcadeButton::Command(ArcadeCommand::NewGame),
        );
    } else {
        for (id, command) in [
            ("arcade-button-attack", ArcadeCommand::Attack),
            ("arcade-button-heal", ArcadeCommand::Heal),
            ("arcade-button-recharge", ArcadeCommand::Recharge),
        ] {
            button(
                &mut commands,
                row,
                text(LocMsg::new(id)),
                ArcadeButton::Command(command),
            );
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        ArcadeButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: ArcadeButton) {
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
    buttons: Query<&ArcadeButton>,
    mut open: ResMut<OpenArcade>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        ArcadeButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Arcade {
                    target: view.target,
                    command,
                });
            }
        }
        ArcadeButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

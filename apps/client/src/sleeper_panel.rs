//! A sleeper console's window (Luna's `sleep_console` window): who lies in the sleeper, how they
//! are, and what to inject.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, SleeperPanel};
use rs13_world::SleeperPanelView;

use crate::{Loc, ui_text_font};

/// The console the server says is open.
#[derive(Resource, Default)]
pub struct OpenSleeper(pub Option<SleeperPanelView>);

#[derive(Component)]
pub struct SleeperWindow;

#[derive(Component, Clone, Copy)]
pub enum SleeperButton {
    Line(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_sleeper(
    mut open: ResMut<OpenSleeper>,
    mut receivers: Query<&mut MessageReceiver<SleeperPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(SleeperPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_sleeper(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenSleeper>,
    windows: Query<Entity, With<SleeperWindow>>,
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
            SleeperWindow,
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
    match &view.occupant {
        None => label(&mut commands, text(LocMsg::new("sleeper-empty"))),
        Some(name) => {
            let state = match view.stat {
                rs13_world::MobStat::Conscious => "conscious",
                rs13_world::MobStat::Unconscious => "unconscious",
                _ => "dead",
            };
            label(
                &mut commands,
                text(
                    LocMsg::new("sleeper-occupant")
                        .arg("name", name.clone())
                        .arg("state", state),
                ),
            );
            label(
                &mut commands,
                text(
                    LocMsg::new("sleeper-readings")
                        .num("value", i64::from(view.health))
                        .num("oxy", i64::from(view.oxy))
                        .num("tox", i64::from(view.tox)),
                ),
            );
            for (line, (reagent, amount)) in view.reagents.iter().enumerate() {
                button(
                    &mut commands,
                    window,
                    text(
                        LocMsg::new("sleeper-inject")
                            .arg("reagent", text(LocMsg::new(format!("reagent-{reagent}"))))
                            .num("value", i64::from(*amount)),
                    ),
                    SleeperButton::Line(line as u8),
                );
            }
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        SleeperButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: SleeperButton) {
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
    buttons: Query<&SleeperButton>,
    mut open: ResMut<OpenSleeper>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        SleeperButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Sleeper {
                    target: view.target,
                    line,
                });
            }
        }
        SleeperButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

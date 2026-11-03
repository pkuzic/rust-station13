//! A body scanner console's window (Luna's `scanconsole` window): who lies in the scanner, how they
//! are, and what's hurt.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, BodyScanPanel, IntentChannel};
use rs13_world::BodyScanView;

use crate::{Loc, ui_text_font};

/// The console the server says is open.
#[derive(Resource, Default)]
pub struct OpenScan(pub Option<BodyScanView>);

#[derive(Component)]
pub struct ScanWindow;

#[derive(Component, Clone, Copy)]
pub enum ScanButton {
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_scan(
    mut open: ResMut<OpenScan>,
    mut receivers: Query<&mut MessageReceiver<BodyScanPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(BodyScanPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_scan(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenScan>,
    windows: Query<Entity, With<ScanWindow>>,
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
            ScanWindow,
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
        None => label(&mut commands, text(LocMsg::new("scanner-empty"))),
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
            if view.parts.is_empty() {
                label(&mut commands, text(LocMsg::new("scanner-unhurt")));
            }
            for (part, brute, burn) in &view.parts {
                label(
                    &mut commands,
                    text(
                        LocMsg::new("scanner-part")
                            .arg("item", part.clone())
                            .num("value", i64::from(*brute))
                            .num("burn", i64::from(*burn)),
                    ),
                );
            }
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        ScanButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: ScanButton) {
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
    buttons: Query<&ScanButton>,
    mut open: ResMut<OpenScan>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        ScanButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

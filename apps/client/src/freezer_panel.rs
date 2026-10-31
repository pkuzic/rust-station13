//! The freezer's window (Luna's `freezer` browser window): on or off, the gas's temperature
//! and pressure, and the temperature it cools to with its six buttons.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, FreezerCommand, FreezerPanel, IntentChannel};
use rs13_world::FreezerPanelView;

use crate::{Loc, ui_text_font};

/// The freezer the server says is open.
#[derive(Resource, Default)]
pub struct OpenFreezer(pub Option<FreezerPanelView>);

#[derive(Component)]
pub struct FreezerWindow;

#[derive(Component, Clone, Copy)]
pub enum FreezerButton {
    Command(FreezerCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
/// Luna's setting buttons: ±100, ±10, ±1 K.
const STEPS: [i32; 3] = [100, 10, 1];

pub fn receive_freezer(
    mut open: ResMut<OpenFreezer>,
    mut receivers: Query<&mut MessageReceiver<FreezerPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(FreezerPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_freezer(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenFreezer>,
    windows: Query<Entity, With<FreezerWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();

    let window = commands
        .spawn((
            FreezerWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    let row = |commands: &mut Commands| {
        commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id()
    };
    let label = |commands: &mut Commands, parent: Entity, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(parent)));
    };

    let status = row(&mut commands);
    let mut args = rs13_i18n::Args::new();
    args.set("on", if view.on { "yes" } else { "no" });
    label(
        &mut commands,
        status,
        loc.0.format("freezer-status", Some(&args)).unwrap_or_default(),
    );
    button(
        &mut commands,
        status,
        text("apc-toggle"),
        FreezerButton::Command(FreezerCommand::Toggle),
    );

    let gas = row(&mut commands);
    let mut args = rs13_i18n::Args::new();
    args.set("temperature", f64::from(view.gas_dk) / 10.0);
    args.set("pressure", f64::from(view.pressure_dkpa) / 10.0);
    label(
        &mut commands,
        gas,
        loc.0.format("freezer-gas", Some(&args)).unwrap_or_default(),
    );

    let setting = row(&mut commands);
    label(&mut commands, setting, text("freezer-setting"));
    for step in STEPS {
        button(
            &mut commands,
            setting,
            "-".into(),
            FreezerButton::Command(FreezerCommand::Setting(-step)),
        );
    }
    label(&mut commands, setting, format!("{} K", view.setting));
    for step in STEPS.into_iter().rev() {
        button(
            &mut commands,
            setting,
            "+".into(),
            FreezerButton::Command(FreezerCommand::Setting(step)),
        );
    }

    let close = row(&mut commands);
    button(&mut commands, close, text("panel-close"), FreezerButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: FreezerButton) {
    commands
        .spawn((
            action,
            Node {
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
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
    buttons: Query<&FreezerButton>,
    mut open: ResMut<OpenFreezer>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        FreezerButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Freezer {
                    target: view.target,
                    command,
                });
            }
        }
        FreezerButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

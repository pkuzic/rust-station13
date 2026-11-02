//! The window of a canister, portable pump or scrubber (Luna's `canister`, `pump` and
//! `scrubber` browser windows): its pressure, the port, the held tank, the valve or power
//! switch, a pump's direction, and the release pressure, target or rate with Luna's buttons.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, GasCommand, GasPanel, IntentChannel};
use rs13_world::{GasMachine, GasPanelView};

use crate::{Loc, ui_text_font};

/// The window the server says is open.
#[derive(Resource, Default)]
pub struct OpenGas(pub Option<GasPanelView>);

#[derive(Component)]
pub struct GasWindow;

#[derive(Component, Clone, Copy)]
pub enum GasButton {
    Command(GasCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
/// Luna's buttons: a canister's and a pump's ±1000, ±100, ±10, ±1 kPa; a scrubber's ±10, ±1 L.
const PRESSURE_STEPS: [i32; 4] = [1000, 100, 10, 1];
const RATE_STEPS: [i32; 2] = [10, 1];

pub fn receive_gas(
    mut open: ResMut<OpenGas>,
    mut receivers: Query<&mut MessageReceiver<GasPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(GasPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_gas(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenGas>,
    windows: Query<Entity, With<GasWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let yes = |b: bool| if b { "yes" } else { "no" };

    let window = commands
        .spawn((
            GasWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(320.0),
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
    let say = |id: &str, a: rs13_i18n::Args| loc.0.format(id, Some(&a)).unwrap_or_default();

    let title = match view.kind {
        GasMachine::Canister => "gas-title-canister",
        GasMachine::Pump => "gas-title-pump",
        GasMachine::Scrubber => "gas-title-scrubber",
    };
    let head = row(&mut commands);
    label(&mut commands, head, text(title));

    let state = row(&mut commands);
    label(
        &mut commands,
        state,
        say("gas-pressure", num("value", f64::from(view.pressure_dkpa) / 10.0)),
    );
    let port = row(&mut commands);
    label(
        &mut commands,
        port,
        say("gas-port", word("connected", yes(view.connected))),
    );
    if let Some(tank) = view.tank_dkpa {
        let held = row(&mut commands);
        label(
            &mut commands,
            held,
            say("gas-tank", num("value", f64::from(tank) / 10.0)),
        );
        button(
            &mut commands,
            held,
            text("gas-remove-tank"),
            GasButton::Command(GasCommand::RemoveTank),
        );
    }

    let switch = row(&mut commands);
    let switch_id = if view.kind == GasMachine::Canister {
        "gas-valve"
    } else {
        "gas-power"
    };
    label(&mut commands, switch, say(switch_id, word("on", yes(view.on))));
    button(
        &mut commands,
        switch,
        text("apc-toggle"),
        GasButton::Command(GasCommand::Toggle),
    );

    if view.kind == GasMachine::Pump {
        let direction = row(&mut commands);
        label(
            &mut commands,
            direction,
            say("gas-direction", word("out", yes(view.out))),
        );
        button(
            &mut commands,
            direction,
            text("apc-toggle"),
            GasButton::Command(GasCommand::Direction),
        );
    }

    let (setting_id, steps, unit): (&str, &[i32], &str) = match view.kind {
        GasMachine::Canister => ("gas-release", &PRESSURE_STEPS, "kPa"),
        GasMachine::Pump => ("gas-target", &PRESSURE_STEPS, "kPa"),
        GasMachine::Scrubber => ("gas-rate", &RATE_STEPS, "L"),
    };
    let setting = row(&mut commands);
    label(&mut commands, setting, text(setting_id));
    for step in steps {
        button(
            &mut commands,
            setting,
            "-".into(),
            GasButton::Command(GasCommand::Adjust(-step)),
        );
    }
    label(&mut commands, setting, format!("{} {unit}", view.setting));
    for step in steps.iter().rev() {
        button(
            &mut commands,
            setting,
            "+".into(),
            GasButton::Command(GasCommand::Adjust(*step)),
        );
    }

    let close = row(&mut commands);
    button(&mut commands, close, text("panel-close"), GasButton::Close);
}

fn num(name: &'static str, value: f64) -> rs13_i18n::Args<'static> {
    let mut a = rs13_i18n::Args::new();
    a.set(name, value);
    a
}

fn word(name: &'static str, value: &'static str) -> rs13_i18n::Args<'static> {
    let mut a = rs13_i18n::Args::new();
    a.set(name, value);
    a
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: GasButton) {
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
    buttons: Query<&GasButton>,
    mut open: ResMut<OpenGas>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        GasButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Gas {
                    target: view.target,
                    command,
                });
            }
        }
        GasButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

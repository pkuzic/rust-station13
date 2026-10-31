//! The SMES window (Luna's `smes` browser window): stored charge, input (charging, its mode
//! and level) and output (on or off, level, load).

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, SmesCommand, SmesLevel, SmesPanel};
use rs13_world::SmesPanelView;

use crate::{Loc, ui_text_font};

/// The SMES the server says is open.
#[derive(Resource, Default)]
pub struct OpenSmes(pub Option<SmesPanelView>);

#[derive(Component)]
pub struct SmesWindow;

#[derive(Component, Clone, Copy)]
pub enum SmesButton {
    Command(SmesCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
/// Luna's level buttons, after "M" (to nothing): −10000, −1000, −100 W; then +100, +1000,
/// +10000 W and "M" (to the most).
const STEPS: [i32; 3] = [10_000, 1_000, 100];

pub fn receive_smes(
    mut open: ResMut<OpenSmes>,
    mut receivers: Query<&mut MessageReceiver<SmesPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(SmesPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_smes(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenSmes>,
    windows: Query<Entity, With<SmesWindow>>,
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
            SmesWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(15.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(340.0),
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
    let say = |id: &str, fill: &dyn Fn(&mut rs13_i18n::Args)| {
        let mut a = rs13_i18n::Args::new();
        fill(&mut a);
        loc.0.format(id, Some(&a)).unwrap_or_default()
    };
    let yes = |b: bool| if b { "yes" } else { "no" };
    let levels = |commands: &mut Commands, parent: Entity, value: u32, make: fn(SmesLevel) -> SmesCommand| {
        button(
            commands,
            parent,
            "M".into(),
            SmesButton::Command(make(SmesLevel::Min)),
        );
        for step in STEPS {
            button(
                commands,
                parent,
                "-".into(),
                SmesButton::Command(make(SmesLevel::Change(-step))),
            );
        }
        commands.spawn((
            Text::new(format!("{value} W")),
            ui_text_font(14.0),
            TextColor(TEXT),
            ChildOf(parent),
        ));
        for step in STEPS.into_iter().rev() {
            button(
                commands,
                parent,
                "+".into(),
                SmesButton::Command(make(SmesLevel::Change(step))),
            );
        }
        button(
            commands,
            parent,
            "M".into(),
            SmesButton::Command(make(SmesLevel::Max)),
        );
    };

    let title = row(&mut commands);
    label(&mut commands, title, text("smes-title"));
    let charge = row(&mut commands);
    label(
        &mut commands,
        charge,
        say("smes-charge", &|a| a.set("value", i64::from(view.charge_percent))),
    );

    let input = row(&mut commands);
    label(
        &mut commands,
        input,
        say("smes-input", &|a| {
            a.set("charging", yes(view.charging));
            a.set("auto", yes(view.charge_auto));
        }),
    );
    button(
        &mut commands,
        input,
        text("apc-toggle"),
        SmesButton::Command(SmesCommand::ChargeMode),
    );
    let input_level = row(&mut commands);
    label(&mut commands, input_level, text("smes-input-level"));
    levels(&mut commands, input_level, view.input, SmesCommand::Input);

    let output = row(&mut commands);
    label(
        &mut commands,
        output,
        say("smes-output", &|a| a.set("on", yes(view.online))),
    );
    button(
        &mut commands,
        output,
        text("apc-toggle"),
        SmesButton::Command(SmesCommand::Online),
    );
    let output_level = row(&mut commands);
    label(&mut commands, output_level, text("smes-output-level"));
    levels(&mut commands, output_level, view.output, SmesCommand::Output);
    let load = row(&mut commands);
    label(
        &mut commands,
        load,
        say("smes-load", &|a| a.set("load", i64::from(view.load))),
    );

    let close = row(&mut commands);
    button(&mut commands, close, text("panel-close"), SmesButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: SmesButton) {
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
    buttons: Query<&SmesButton>,
    mut open: ResMut<OpenSmes>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        SmesButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Smes {
                    target: view.target,
                    command,
                });
            }
        }
        SmesButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

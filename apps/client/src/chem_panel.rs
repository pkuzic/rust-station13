//! A chem dispenser's window (Luna's `chem_dispenser` window): its energy, its beaker, how much
//! to give at a time, and what it can give.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ChemCommand, ChemPanel, IntentChannel};
use rs13_world::ChemPanelView;

use crate::{Loc, ui_text_font};

/// The dispenser the server says is open.
#[derive(Resource, Default)]
pub struct OpenChem(pub Option<ChemPanelView>);

#[derive(Component)]
pub struct ChemWindow;

#[derive(Component, Clone, Copy)]
pub enum ChemButton {
    Command(ChemCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_chem(
    mut open: ResMut<OpenChem>,
    mut receivers: Query<&mut MessageReceiver<ChemPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ChemPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_chem(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenChem>,
    windows: Query<Entity, With<ChemWindow>>,
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
            ChemWindow,
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
        text(LocMsg::new("dispenser-title").num("value", i64::from(view.energy))),
    );
    match view.beaker {
        Some((held, size)) => {
            label(
                &mut commands,
                text(
                    LocMsg::new("dispenser-beaker")
                        .num("value", i64::from(held))
                        .num("max", i64::from(size)),
                ),
            );
            button(
                &mut commands,
                window,
                text(LocMsg::new("dispenser-eject")),
                ChemButton::Command(ChemCommand::Eject),
            );
        }
        None => label(&mut commands, text(LocMsg::new("dispenser-no-beaker"))),
    }
    let amounts = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for amount in [5u8, 10, 30] {
        let shown = if view.amount == amount {
            format!("[{amount}]")
        } else {
            amount.to_string()
        };
        button(
            &mut commands,
            amounts,
            shown,
            ChemButton::Command(ChemCommand::Amount(amount)),
        );
    }
    // In two columns: there are many, and the window has to fit the map beside it.
    let columns = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for (line, reagent) in view.reagents.iter().enumerate() {
        button(
            &mut commands,
            columns,
            text(LocMsg::new(format!("reagent-{reagent}"))),
            ChemButton::Command(ChemCommand::Dispense(line as u8)),
        );
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        ChemButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: ChemButton) {
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
    buttons: Query<&ChemButton>,
    mut open: ResMut<OpenChem>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        ChemButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Chem {
                    target: view.target,
                    command,
                });
            }
        }
        ChemButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

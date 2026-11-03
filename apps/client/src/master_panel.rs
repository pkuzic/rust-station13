//! A CheMaster's window (Luna's `chemmaster` window): what's in its beaker and its buffer, and
//! buttons to move them and make pills and bottles.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ChemMasterCommand, ChemMasterPanel, IntentChannel};
use rs13_world::ChemMasterPanelView;

use crate::{Loc, ui_text_font};

/// The CheMaster the server says is open.
#[derive(Resource, Default)]
pub struct OpenMaster(pub Option<ChemMasterPanelView>);

#[derive(Component)]
pub struct MasterWindow;

#[derive(Component, Clone, Copy)]
pub enum MasterButton {
    Command(ChemMasterCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_master(
    mut open: ResMut<OpenMaster>,
    mut receivers: Query<&mut MessageReceiver<ChemMasterPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ChemMasterPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_master(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenMaster>,
    windows: Query<Entity, With<MasterWindow>>,
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
            MasterWindow,
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
    label(&mut commands, text(LocMsg::new("master-title")));
    let line = |reagent: &str, value: u16| {
        text(
            LocMsg::new("master-line")
                .arg("reagent", text(LocMsg::new(format!("reagent-{reagent}"))))
                .num("value", i64::from(value)),
        )
    };
    match &view.beaker {
        None => label(&mut commands, text(LocMsg::new("dispenser-no-beaker"))),
        Some(beaker) => {
            button(
                &mut commands,
                window,
                text(LocMsg::new("master-eject")),
                MasterButton::Command(ChemMasterCommand::Eject),
            );
            label(&mut commands, text(LocMsg::new("master-to-buffer")));
            for (n, (reagent, value)) in beaker.iter().enumerate() {
                button(
                    &mut commands,
                    window,
                    line(reagent, *value),
                    MasterButton::Command(ChemMasterCommand::Add(n as u8)),
                );
            }
            label(&mut commands, text(LocMsg::new("master-to-beaker")));
            for (n, (reagent, value)) in view.buffer.iter().enumerate() {
                button(
                    &mut commands,
                    window,
                    line(reagent, *value),
                    MasterButton::Command(ChemMasterCommand::Remove(n as u8)),
                );
            }
            button(
                &mut commands,
                window,
                text(LocMsg::new("master-pill")),
                MasterButton::Command(ChemMasterCommand::Pill),
            );
            button(
                &mut commands,
                window,
                text(LocMsg::new("master-bottle")),
                MasterButton::Command(ChemMasterCommand::Bottle),
            );
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        MasterButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: MasterButton) {
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
    buttons: Query<&MasterButton>,
    mut open: ResMut<OpenMaster>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        MasterButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::ChemMaster {
                    target: view.target,
                    command,
                });
            }
        }
        MasterButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

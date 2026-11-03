//! An identification computer's window (Luna's `computer/card` window): the card in it, and the
//! jobs to give it.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IdComputerPanel, IntentChannel};
use rs13_world::IdComputerView;

use crate::{Loc, ui_text_font};

/// The identification computer the server says is open.
#[derive(Resource, Default)]
pub struct OpenIdComputer(pub Option<IdComputerView>);

#[derive(Component)]
pub struct IdWindow;

#[derive(Component, Clone, Copy)]
pub enum IdButton {
    Line(u8),
    Eject,
    Manifest,
    Print,
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_id(
    mut open: ResMut<OpenIdComputer>,
    mut receivers: Query<&mut MessageReceiver<IdComputerPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(IdComputerPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_id(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenIdComputer>,
    windows: Query<Entity, With<IdWindow>>,
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
            IdWindow,
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
    if let Some(crew) = &view.manifest {
        label(&mut commands, text(LocMsg::new("id-manifest-title")));
        label(&mut commands, text(LocMsg::new("id-manifest-hint")));
        for (name, job) in crew {
            let job = text(LocMsg::new(format!("job-{job}")));
            label(
                &mut commands,
                text(
                    LocMsg::new("id-manifest-row")
                        .arg("name", name.clone())
                        .arg("job", job),
                ),
            );
        }
        if view.printing {
            label(&mut commands, text(LocMsg::new("id-manifest-printing")));
        } else {
            button(
                &mut commands,
                window,
                text(LocMsg::new("id-manifest-print")),
                IdButton::Print,
            );
        }
        button(
            &mut commands,
            window,
            text(LocMsg::new("id-manifest-back")),
            IdButton::Manifest,
        );
    } else if !view.has_card {
        label(&mut commands, text(LocMsg::new("id-computer-empty")));
    } else {
        let job = view.job.as_ref().map_or_else(
            || text(LocMsg::new("id-computer-no-job")),
            |job| text(LocMsg::new(format!("job-{job}"))),
        );
        label(
            &mut commands,
            text(
                LocMsg::new("id-computer-card")
                    .arg("name", view.owner.clone().unwrap_or_else(|| "-".to_owned()))
                    .arg("job", job),
            ),
        );
        button(
            &mut commands,
            window,
            text(LocMsg::new("id-computer-eject")),
            IdButton::Eject,
        );
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
        for (line, job) in view.jobs.iter().enumerate() {
            button(
                &mut commands,
                columns,
                text(LocMsg::new(format!("job-{job}"))),
                IdButton::Line(line as u8),
            );
        }
    }
    if view.manifest.is_none() {
        button(
            &mut commands,
            window,
            text(LocMsg::new("id-manifest-open")),
            IdButton::Manifest,
        );
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        IdButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: IdButton) {
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
    buttons: Query<&IdButton>,
    mut open: ResMut<OpenIdComputer>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        IdButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::IdComputer {
                    target: view.target,
                    line: Some(line),
                });
            }
        }
        IdButton::Eject => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::IdComputer {
                    target: view.target,
                    line: None,
                });
            }
        }
        IdButton::Manifest | IdButton::Print => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::IdManifest {
                    target: view.target,
                    print: matches!(*action, IdButton::Print),
                });
            }
        }
        IdButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

//! The changeling's window: F6 asks for it, and only a changeling gets one. Spec:
//! `docs/reference/changeling.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ChangelingCommand, ChangelingPanel, IntentChannel};
use rs13_world::{ChangelingPanelView, NetId};

use crate::{ChatInput, ChatKind, Loc, ui_text_font};

/// The window, if it's open, and the power being aimed.
#[derive(Resource, Default)]
pub struct OpenLing {
    pub view: Option<ChangelingPanelView>,
    picking: Option<u8>,
    dna: Option<u8>,
}

#[derive(Component)]
pub struct LingWindow;

#[derive(Component, Clone, Copy)]
pub enum LingButton {
    Power(u8),
    Buy(u8),
    Target(NetId),
    Dna(u8),
    Cancel,
    GiveUp,
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

/// F6: ask for the window, or close it.
pub fn toggle_ling(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    if chat.open || !keys.just_pressed(KeyCode::F6) {
        return;
    }
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(ActionInput::Changeling(ChangelingCommand::Open));
    }
}

pub fn receive_ling(
    mut open: ResMut<OpenLing>,
    mut receivers: Query<&mut MessageReceiver<ChangelingPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ChangelingPanel(view)) = receiver.receive().last() {
            if view.is_none() {
                open.picking = None;
                open.dna = None;
            }
            open.view = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_ling(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenLing>,
    windows: Query<Entity, With<LingWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.view else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let window = commands
        .spawn((
            LingWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(8.0),
                top: Val::VMin(3.0),
                min_width: Val::Px(440.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(2.0),
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
    label(&mut commands, text(LocMsg::new("ling-title")));
    label(
        &mut commands,
        text(
            LocMsg::new("ling-stats")
                .num("value", i64::from(view.chems))
                .num("storage", i64::from(view.storage))
                .num("points", i64::from(view.points))
                .num("absorbed", i64::from(view.absorbed))
                .num("damage", i64::from(view.genetic_damage)),
        ),
    );
    if let Some(name) = &view.mimic {
        label(
            &mut commands,
            text(LocMsg::new("ling-mimicking").arg("name", name.clone())),
        );
    }
    if view.dead {
        button(
            &mut commands,
            window,
            text(LocMsg::new("ling-give-up")),
            LingButton::GiveUp,
        );
    }
    if let Some(line) = open.picking {
        let power = view.powers.get(usize::from(line));
        let wants_dna = power.is_some_and(|p| p.needs_dna) && open.dna.is_none();
        let wants_target = power.is_some_and(|p| p.needs_target);
        label(&mut commands, text(LocMsg::new("ling-choose")));
        if wants_dna {
            let list = if power.is_some_and(|p| p.id == "hive-absorb") {
                &view.hive
            } else {
                &view.dna
            };
            for (i, name) in list.iter().enumerate() {
                button(&mut commands, window, name.clone(), LingButton::Dna(i as u8));
            }
        } else if wants_target {
            for (id, name) in &view.nearby {
                button(&mut commands, window, name.clone(), LingButton::Target(*id));
            }
        }
        button(
            &mut commands,
            window,
            text(LocMsg::new("ling-cancel")),
            LingButton::Cancel,
        );
        return;
    }
    let columns = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for (line, power) in view.powers.iter().enumerate() {
        let name = LocMsg::new(format!("ling-power-{}", power.id));
        if !power.owned {
            let shown = text(
                LocMsg::new("ling-evolve")
                    .msg("name", format!("ling-power-{}", power.id))
                    .num("value", i64::from(power.cost)),
            );
            button(&mut commands, columns, shown, LingButton::Buy(line as u8));
        } else if power.passive {
            commands.spawn((
                Text::new(text(name)),
                ui_text_font(14.0),
                TextColor(TEXT),
                ChildOf(columns),
            ));
        } else {
            let shown = text(
                LocMsg::new("ling-use")
                    .msg("name", format!("ling-power-{}", power.id))
                    .num("value", i64::from(power.chems)),
            );
            button(&mut commands, columns, shown, LingButton::Power(line as u8));
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        LingButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: LingButton) {
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
    buttons: Query<&LingButton>,
    mut open: ResMut<OpenLing>,
    mut chat: ResMut<ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let mut send = |command: ChangelingCommand| {
        sender.send::<IntentChannel>(ActionInput::Changeling(command));
    };
    let power_at = |open: &OpenLing, line: u8| {
        open.view
            .as_ref()
            .and_then(|view| view.powers.get(usize::from(line)).cloned())
    };
    match *action {
        LingButton::Power(line) => {
            let Some(power) = power_at(&open, line) else {
                return;
            };
            let mimicking = open.view.as_ref().is_some_and(|view| view.mimic.is_some());
            if power.needs_text && !mimicking {
                chat.open = true;
                chat.kind = ChatKind::Mimic;
                chat.buffer.clear();
            } else if power.needs_target || power.needs_dna {
                open.picking = Some(line);
                open.dna = None;
            } else {
                send(ChangelingCommand::Use {
                    power: line,
                    target: None,
                    dna: None,
                });
            }
        }
        LingButton::Buy(line) => send(ChangelingCommand::Buy(line)),
        LingButton::Dna(dna) => {
            let Some(line) = open.picking else { return };
            let wants_target = power_at(&open, line).is_some_and(|p| p.needs_target);
            if wants_target {
                open.dna = Some(dna);
            } else {
                send(ChangelingCommand::Use {
                    power: line,
                    target: None,
                    dna: Some(dna),
                });
                open.picking = None;
            }
        }
        LingButton::Target(target) => {
            let Some(line) = open.picking else { return };
            let dna = open.dna;
            send(ChangelingCommand::Use {
                power: line,
                target: Some(target),
                dna,
            });
            open.picking = None;
            open.dna = None;
        }
        LingButton::Cancel => {
            open.picking = None;
            open.dna = None;
        }
        LingButton::GiveUp => send(ChangelingCommand::GiveUp),
        LingButton::Close => send(ChangelingCommand::Open),
    }
}

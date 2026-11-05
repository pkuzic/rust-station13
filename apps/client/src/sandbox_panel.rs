//! The sandbox panel (Luna's `h_sandbox`): F5 asks for it, and the server only answers in a sandbox round.
//! Spec: `docs/reference/round.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, SandboxCommand, SandboxPanel};
use rs13_world::SandboxPanelView;

use crate::{ChatInput, Loc, ui_text_font};

/// How many items one page of the list shows.
const PAGE: usize = 24;

/// The panel, if it's open, and the page of items showing.
#[derive(Resource, Default)]
pub struct OpenSandbox {
    pub view: Option<SandboxPanelView>,
    page: usize,
}

#[derive(Component)]
pub struct SandboxWindow;

#[derive(Component, Clone, Copy)]
pub enum SandboxButton {
    Do(SandboxCommand),
    Page(bool),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

/// F5: ask for the panel, or close it.
pub fn toggle_sandbox(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    mut open: ResMut<OpenSandbox>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    if chat.open || !keys.just_pressed(KeyCode::F5) {
        return;
    }
    if open.view.is_some() {
        open.view = None;
    } else if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(ActionInput::Sandbox(SandboxCommand::Open));
    }
}

pub fn receive_sandbox(
    mut open: ResMut<OpenSandbox>,
    mut receivers: Query<&mut MessageReceiver<SandboxPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(SandboxPanel(view)) = receiver.receive().last() {
            open.view = Some(view);
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_sandbox(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenSandbox>,
    windows: Query<Entity, With<SandboxWindow>>,
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
            SandboxWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(10.0),
                top: Val::VMin(4.0),
                min_width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    commands.spawn((
        Text::new(text(LocMsg::new("sandbox-title"))),
        ui_text_font(14.0),
        TextColor(TEXT),
        ChildOf(window),
    ));
    button(
        &mut commands,
        window,
        text(LocMsg::new(view.suit.clone())),
        SandboxButton::Do(SandboxCommand::Suit),
    );
    for (line, (id, amount)) in view.kits.iter().enumerate() {
        let name = LocMsg::new("sandbox-kit")
            .arg("item", id.clone())
            .num("value", i64::from(*amount));
        button(
            &mut commands,
            window,
            text(name),
            SandboxButton::Do(SandboxCommand::Kit(line as u8)),
        );
    }
    if !view.objects.is_empty() {
        let pages = view.objects.len().div_ceil(PAGE);
        let page = open.page.min(pages - 1);
        commands.spawn((
            Text::new(text(
                LocMsg::new("sandbox-objects")
                    .num("value", page as i64 + 1)
                    .num("pages", pages as i64),
            )),
            ui_text_font(14.0),
            TextColor(TEXT),
            ChildOf(window),
        ));
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
        for (line, id) in view.objects.iter().enumerate().skip(page * PAGE).take(PAGE) {
            button(
                &mut commands,
                columns,
                text(LocMsg::new("sandbox-object").arg("item", id.clone())),
                SandboxButton::Do(SandboxCommand::Spawn(line as u16)),
            );
        }
        button(
            &mut commands,
            window,
            text(LocMsg::new("sandbox-previous")),
            SandboxButton::Page(false),
        );
        button(
            &mut commands,
            window,
            text(LocMsg::new("sandbox-next")),
            SandboxButton::Page(true),
        );
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        SandboxButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: SandboxButton) {
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
    buttons: Query<&SandboxButton>,
    mut open: ResMut<OpenSandbox>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    match *action {
        SandboxButton::Do(command) => {
            if let Ok(mut sender) = sender.single_mut() {
                sender.send::<IntentChannel>(ActionInput::Sandbox(command));
            }
        }
        SandboxButton::Page(forward) => {
            let pages = open
                .view
                .as_ref()
                .map_or(1, |view| view.objects.len().div_ceil(PAGE))
                .max(1);
            open.page = if forward {
                (open.page + 1) % pages
            } else {
                (open.page + pages - 1) % pages
            };
        }
        SandboxButton::Close => open.view = None,
    }
}

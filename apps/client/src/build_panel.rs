//! What a stack of sheets can make (Luna's stack recipes). Spec: `docs/reference/construction.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, BuildPanel, IntentChannel};
use rs13_world::BuildPanelView;

use crate::{Loc, ui_text_font};

/// The stack the server says is open.
#[derive(Resource, Default)]
pub struct OpenBuild(pub Option<BuildPanelView>);

#[derive(Component)]
pub struct BuildWindow;

#[derive(Component, Clone, Copy)]
pub enum BuildButton {
    Make(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_build(
    mut open: ResMut<OpenBuild>,
    mut receivers: Query<&mut MessageReceiver<BuildPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(BuildPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_build(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenBuild>,
    windows: Query<Entity, With<BuildWindow>>,
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
            BuildWindow,
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
        text(LocMsg::new("build-title").num("value", i64::from(view.amount))),
    );
    for (line, (proto, cost, made)) in view.recipes.iter().enumerate() {
        let id = if *made > 1 {
            "build-recipe-many"
        } else {
            "build-recipe"
        };
        let name = LocMsg::new(id)
            .arg("item", proto.clone())
            .num("cost", i64::from(*cost))
            .num("made", i64::from(*made));
        if u16::from(*cost) <= view.amount {
            button(&mut commands, window, text(name), BuildButton::Make(line as u8));
        } else {
            label(&mut commands, text(name));
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        BuildButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: BuildButton) {
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
    buttons: Query<&BuildButton>,
    mut open: ResMut<OpenBuild>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        BuildButton::Make(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Build {
                    target: view.target,
                    line,
                });
            }
        }
        BuildButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

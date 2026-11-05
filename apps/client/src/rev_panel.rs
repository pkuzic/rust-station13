//! What revolutionaries see: the R over each other's heads (Luna's `update_rev_icons`), and the question
//! an RP revolution asks. Spec: `docs/reference/revolution.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, RevAskPanel, RevIcons};
use rs13_world::NetId;

use crate::sprites::SpriteAtlas;
use crate::{Loc, ui_text_font};

/// Who's in on it, as far as we're told, and whether each leads. Empty if we aren't in on it.
#[derive(Resource, Default)]
pub struct RevMembers(pub Vec<(NetId, bool)>);

/// Who's asking us to join, if anyone is.
#[derive(Resource, Default)]
pub struct RevQuestion(pub Option<String>);

/// An R over someone's head.
#[derive(Component)]
pub struct RevMark(bool);

#[derive(Component)]
pub struct RevWindow;

#[derive(Component, Clone, Copy)]
pub struct RevButton(bool);

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_rev(
    mut members: ResMut<RevMembers>,
    mut question: ResMut<RevQuestion>,
    mut icons: Query<&mut MessageReceiver<RevIcons>, With<Client>>,
    mut asks: Query<&mut MessageReceiver<RevAskPanel>, With<Client>>,
) {
    for mut receiver in &mut icons {
        if let Some(RevIcons(list)) = receiver.receive().last() {
            members.0 = list;
        }
    }
    for mut receiver in &mut asks {
        if let Some(RevAskPanel(asker)) = receiver.receive().last() {
            question.0 = asker;
        }
    }
}

/// Keep an R on everyone we're told of, and on no one else.
pub fn draw_rev_marks(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    members: Res<RevMembers>,
    people: Query<(Entity, &NetId)>,
    marks: Query<(Entity, &ChildOf, &RevMark)>,
) {
    let Some(atlas) = atlas else { return };
    let wanted = |person: Entity| {
        people
            .get(person)
            .ok()
            .and_then(|(_, id)| members.0.iter().find(|(member, _)| member == id))
            .map(|(_, leads)| *leads)
    };
    for (mark, parent, RevMark(leads)) in &marks {
        if wanted(parent.parent()) != Some(*leads) {
            commands.entity(mark).despawn();
        }
    }
    for (person, _) in &people {
        let Some(leads) = wanted(person) else { continue };
        if marks
            .iter()
            .any(|(_, parent, RevMark(l))| parent.parent() == person && *l == leads)
        {
            continue;
        }
        let state = if leads { "head" } else { "" };
        if let Some(image) = atlas.sprite("rev-mark", state, 0) {
            commands
                .entity(person)
                .with_child((image, Transform::from_xyz(0.0, 0.0, 0.5), RevMark(leads)));
        }
    }
}

/// The question, while it's there to answer.
pub fn show_question(
    mut commands: Commands,
    loc: Res<Loc>,
    question: Res<RevQuestion>,
    windows: Query<Entity, With<RevWindow>>,
) {
    if !question.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(asker) = &question.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let window = commands
        .spawn((
            RevWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(20.0),
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
    commands.spawn((
        Text::new(text(LocMsg::new("rev-question").arg("name", asker.clone()))),
        ui_text_font(14.0),
        TextColor(TEXT),
        ChildOf(window),
    ));
    for (id, yes) in [("rev-answer-yes", true), ("rev-answer-no", false)] {
        commands
            .spawn((
                RevButton(yes),
                Node {
                    padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                    ..default()
                },
                Hovered::default(),
                ChildOf(window),
            ))
            .with_child((
                Text::new(text(LocMsg::new(id))),
                ui_text_font(14.0),
                TextColor(LINK),
                Pickable::IGNORE,
            ))
            .observe(click_button);
    }
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&RevButton>,
    mut question: ResMut<RevQuestion>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(RevButton(yes)) = buttons.get(click.entity) else {
        return;
    };
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(ActionInput::RevAnswer(*yes));
    }
    question.0 = None;
}

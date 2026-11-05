//! The lobby (Luna's `new_player` window): the countdown, ready or join, your name, and the job you'd
//! like. Spec: `docs/reference/round.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, LobbyPanel};
use rs13_world::{CharacterPrefs, FACIAL_STYLES, HAIR_STYLES, LobbyView, UNDERWEAR_KINDS};

use crate::{Loc, ui_text_font};

/// The lobby the server says we are in.
#[derive(Resource, Default)]
pub struct OpenLobby(pub Option<LobbyView>);

#[derive(Component)]
pub struct LobbyWindow;

#[derive(Component, Clone, Copy)]
pub enum LobbyButton {
    Ready(bool),
    Prefer(u8),
    Join,
    Vote,
    Observe,
    Crew,
    Pref(Pref),
}

/// A line of the character setup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pref {
    Gender,
    SkinDarker,
    SkinLighter,
    Hair,
    Facial,
    HairColour,
    EyeColour,
    Underwear,
    Traitor,
    Slot,
}

/// The colours on offer for hair and eyes, by message id.
const COLOURS: [(&str, [u8; 3]); 8] = [
    ("black", [16, 16, 16]),
    ("brown", [90, 50, 20]),
    ("blonde", [220, 190, 110]),
    ("red", [170, 50, 20]),
    ("grey", [140, 140, 140]),
    ("white", [235, 235, 235]),
    ("blue", [40, 80, 200]),
    ("green", [40, 150, 60]),
];

/// What the player has set up, and where it's kept.
#[derive(Resource)]
pub struct Prefs {
    pub data: CharacterPrefs,
    path: std::path::PathBuf,
    /// Which of the three job choices a click sets.
    slot: usize,
    show_crew: bool,
}

impl Prefs {
    pub fn load(given: Option<String>) -> Self {
        let path = given.map_or_else(
            || {
                let home = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"));
                home.map_or_else(std::path::PathBuf::new, std::path::PathBuf::from)
                    .join("rs13-prefs.ron")
            },
            std::path::PathBuf::from,
        );
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            data,
            path,
            slot: 0,
            show_crew: false,
        }
    }

    fn save(&self) {
        let Ok(text) = ron::to_string(&self.data) else {
            return;
        };
        if let Err(error) = std::fs::write(&self.path, text) {
            warn!(%error, "couldn't save the character setup");
        }
    }

    fn change(&mut self, line: Pref) {
        let look = &mut self.data.look;
        let next_colour = |now: [u8; 3]| {
            let at = COLOURS.iter().position(|(_, c)| *c == now).unwrap_or(0);
            COLOURS[(at + 1) % COLOURS.len()].1
        };
        match line {
            Pref::Gender => look.female = !look.female,
            Pref::SkinDarker => look.skin = (look.skin - 20).max(-180),
            Pref::SkinLighter => look.skin = (look.skin + 20).min(0),
            Pref::Hair => look.hair = (look.hair + 1) % (HAIR_STYLES.len() as u8 + 1),
            Pref::Facial => look.facial = (look.facial + 1) % (FACIAL_STYLES.len() as u8 + 1),
            Pref::HairColour => look.hair_rgb = next_colour(look.hair_rgb),
            Pref::EyeColour => look.eyes_rgb = next_colour(look.eyes_rgb),
            Pref::Underwear => look.underwear = (look.underwear + 1) % (UNDERWEAR_KINDS + 1),
            Pref::Traitor => self.data.be_traitor = !self.data.be_traitor,
            Pref::Slot => self.slot = (self.slot + 1) % self.data.jobs.len(),
        }
    }
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);
const GREY: Color = Color::srgb(0.45, 0.45, 0.45);

pub fn receive_lobby(
    mut open: ResMut<OpenLobby>,
    mut receivers: Query<&mut MessageReceiver<LobbyPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(LobbyPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_lobby(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenLobby>,
    prefs: Res<Prefs>,
    windows: Query<Entity, With<LobbyWindow>>,
) {
    if !open.is_changed() && !prefs.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    // Over everything: there's nothing to see until you're in the world.
    let backdrop = commands
        .spawn((
            LobbyWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                // The chat stays in view, beside the map.
                width: Val::VMin(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Hovered::default(),
            GlobalZIndex(9),
        ))
        .id();
    let window = commands
        .spawn((
            Node {
                min_width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            ChildOf(backdrop),
        ))
        .id();
    let label = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(window)));
    };
    label(&mut commands, text(LocMsg::new("lobby-title")));
    match view.starts_in {
        Some(seconds) => {
            label(
                &mut commands,
                text(
                    LocMsg::new("lobby-countdown")
                        .num("value", i64::from(seconds))
                        .num("ready", i64::from(view.ready_count))
                        .num("players", i64::from(view.players)),
                ),
            );
            let (id, next) = if view.ready {
                ("lobby-unready", false)
            } else {
                ("lobby-ready", true)
            };
            button(
                &mut commands,
                window,
                text(LocMsg::new(id)),
                LobbyButton::Ready(next),
            );
        }
        None => {
            label(&mut commands, text(LocMsg::new("lobby-round-on")));
            button(
                &mut commands,
                window,
                text(LocMsg::new("lobby-join")),
                LobbyButton::Join,
            );
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("lobby-vote")),
        LobbyButton::Vote,
    );
    label(
        &mut commands,
        match &view.name {
            Some(name) => text(LocMsg::new("lobby-name").arg("name", name.clone())),
            None => text(LocMsg::new("lobby-no-name")),
        },
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("lobby-observe")),
        LobbyButton::Observe,
    );
    if !view.crew.is_empty() {
        button(
            &mut commands,
            window,
            text(LocMsg::new("lobby-crew")),
            LobbyButton::Crew,
        );
    }
    if prefs.show_crew {
        for (name, job) in &view.crew {
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
    }
    let look = &prefs.data.look;
    let colour_of = |rgb: [u8; 3]| {
        let id = COLOURS
            .iter()
            .find(|(_, c)| *c == rgb)
            .map_or("black", |(id, _)| id);
        text(LocMsg::new(format!("colour-{id}")))
    };
    let hair = usize::from(look.hair)
        .checked_sub(1)
        .and_then(|line| HAIR_STYLES.get(line))
        .map_or_else(
            || text(LocMsg::new("hair-bald")),
            |style| text(LocMsg::new(format!("hair-{style}"))),
        );
    let facial = usize::from(look.facial)
        .checked_sub(1)
        .and_then(|line| FACIAL_STYLES.get(line))
        .map_or_else(
            || text(LocMsg::new("hair-shaved")),
            |style| text(LocMsg::new(format!("hair-{style}"))),
        );
    let gender = if look.female { "pref-female" } else { "pref-male" };
    let traitor = if prefs.data.be_traitor {
        "pref-traitor-on"
    } else {
        "pref-traitor-off"
    };
    let lines = [
        (text(LocMsg::new(gender)), Pref::Gender),
        (text(LocMsg::new("pref-hair").arg("what", hair)), Pref::Hair),
        (
            text(LocMsg::new("pref-hair-colour").arg("what", colour_of(look.hair_rgb))),
            Pref::HairColour,
        ),
        (text(LocMsg::new("pref-facial").arg("what", facial)), Pref::Facial),
        (
            text(LocMsg::new("pref-eyes").arg("what", colour_of(look.eyes_rgb))),
            Pref::EyeColour,
        ),
        (
            text(LocMsg::new("pref-darker").num("value", i64::from(-look.skin))),
            Pref::SkinDarker,
        ),
        (text(LocMsg::new("pref-lighter")), Pref::SkinLighter),
        (
            text(LocMsg::new("pref-underwear").num("value", i64::from(look.underwear))),
            Pref::Underwear,
        ),
        (text(LocMsg::new(traitor)), Pref::Traitor),
    ];
    let setup = commands
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
    for (shown, line) in lines {
        button(&mut commands, setup, shown, LobbyButton::Pref(line));
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("pref-job-slot").num("value", prefs.slot as i64 + 1)),
        LobbyButton::Pref(Pref::Slot),
    );
    label(&mut commands, text(LocMsg::new("lobby-prefer")));
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
    for (line, (job, open)) in view.jobs.iter().enumerate() {
        let name = text(LocMsg::new(format!("job-{job}")));
        let rank = prefs.data.jobs.iter().position(|j| *j == Some(line as u8));
        let chosen = view.preference.as_ref() == Some(job);
        if *open > 0 {
            let shown = text(
                LocMsg::new("job-row")
                    .arg("job", name)
                    .num("value", i64::from(*open)),
            );
            let shown = match rank {
                Some(rank) => format!("{}. {shown}", rank + 1),
                None if chosen => format!("[{shown}]"),
                None => shown,
            };
            button(&mut commands, columns, shown, LobbyButton::Prefer(line as u8));
        } else {
            commands.spawn((
                Text::new(name),
                ui_text_font(14.0),
                TextColor(GREY),
                ChildOf(columns),
            ));
        }
    }
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: LobbyButton) {
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
    buttons: Query<&LobbyButton>,
    mut votes: ResMut<crate::vote_panel::OpenVote>,
    mut prefs: ResMut<Prefs>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    if matches!(*action, LobbyButton::Vote) {
        votes.open = true;
        return;
    }
    if matches!(*action, LobbyButton::Crew) {
        prefs.show_crew = !prefs.show_crew;
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    if let LobbyButton::Pref(line) = *action {
        prefs.change(line);
        prefs.save();
        sender.send::<IntentChannel>(ActionInput::SetPrefs(prefs.data));
        return;
    }
    if let LobbyButton::Prefer(line) = *action {
        let slot = prefs.slot;
        prefs.data.jobs[slot] = Some(line);
        prefs.save();
        sender.send::<IntentChannel>(ActionInput::SetPrefs(prefs.data));
    }
    sender.send::<IntentChannel>(match *action {
        LobbyButton::Ready(ready) => ActionInput::Ready(ready),
        LobbyButton::Prefer(line) => ActionInput::ChooseJob { line: Some(line) },
        LobbyButton::Join => ActionInput::JoinRound,
        LobbyButton::Observe => ActionInput::Observe,
        LobbyButton::Vote | LobbyButton::Crew | LobbyButton::Pref(_) => return,
    });
}

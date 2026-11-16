//! Paper: reading a sheet, its text box (Luna's `input(... as message)`) and its label box,
//! and the clipboard's window. Writing and labelling type into the say line's buffer
//! (`ChatInput`), so movement and hotkeys stay quiet while you type. Spec:
//! `docs/reference/paperwork.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, ClipboardCommand, ClipboardPanel, IntentChannel, PaperPanel};
use rs13_world::{ClipboardView, PaperMode, PaperView};

use crate::{ChatInput, Loc, ui_text_font};
use rs13_i18n::Localizer;

/// The sheet the server has opened for us.
#[derive(Resource, Default)]
pub struct OpenPaper(pub Option<PaperView>);

/// The clipboard the server says is open.
#[derive(Resource, Default)]
pub struct OpenClipboard(pub Option<ClipboardView>);

#[derive(Component)]
pub struct PaperWindow;

#[derive(Component)]
pub struct ClipboardWindow;

#[derive(Component, Clone, Copy)]
pub enum PaperButton {
    Save,
    Close,
}

#[derive(Component, Clone, Copy)]
pub enum ClipboardButton {
    Command(ClipboardCommand),
    Close,
}

/// Longest text and label (Luna 3 071 and 31 characters).
pub const MAX_TEXT: usize = 3071;
pub const MAX_LABEL: usize = 31;

const FACE: Color = Color::srgb(0.96, 0.95, 0.88);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const INK: Color = Color::srgb(0.05, 0.05, 0.2);

/// What a printed sheet says in the reader's language: `[msg:ID]` is that message, `[job:ID]`
/// that job's name and `[noun:ID]` that thing's name. Anything else in brackets stays as it is.
fn filled_in(loc: &Localizer, text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let id = rest.find(']').and_then(|end| {
            let inside = &rest[1..end];
            let id = inside
                .strip_prefix("msg:")
                .map(str::to_owned)
                .or_else(|| inside.strip_prefix("job:").map(|job| format!("job-{job}")))
                .or_else(|| inside.strip_prefix("noun:").map(|noun| format!("\u{1}{noun}")))?;
            Some((id, end))
        });
        let said = |id: &str| match id.strip_prefix('\u{1}') {
            Some(noun) => Some(loc.noun(noun, rs13_i18n::Case::Nom)),
            None => loc.format(id, None).ok(),
        };
        match id.and_then(|(id, end)| Some((said(&id)?, end))) {
            Some((said, end)) => {
                out.push_str(&said);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('[');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Luna's paper markup, shown as plain text: [b], [i] and [u] tags are dropped.
pub fn plain(text: &str) -> String {
    ["[b]", "[/b]", "[i]", "[/i]", "[u]", "[/u]"]
        .iter()
        .fold(text.to_owned(), |t, tag| t.replace(tag, ""))
}

pub fn receive_paper(
    mut open: ResMut<OpenPaper>,
    mut chat: ResMut<ChatInput>,
    mut receivers: Query<&mut MessageReceiver<PaperPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(PaperPanel(view)) = receiver.receive().last() {
            // Writing edits the whole text, as Luna's box does; a label starts empty.
            match view.mode {
                PaperMode::Read => {}
                PaperMode::Write => chat.start_paper(view.target, view.mode, view.text.clone()),
                PaperMode::Label => chat.start_paper(view.target, view.mode, String::new()),
            }
            open.0 = Some(view);
        }
    }
}

pub fn receive_clipboard(
    mut open: ResMut<OpenClipboard>,
    mut receivers: Query<&mut MessageReceiver<ClipboardPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(ClipboardPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

fn window_node() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::VMin(18.0),
            top: Val::VMin(6.0),
            min_width: Val::Px(360.0),
            max_width: Val::Px(520.0),
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
        GlobalZIndex(12),
    )
}

/// Rebuild the paper window whenever the sheet or what is being typed changes.
pub fn show_paper(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenPaper>,
    chat: Res<ChatInput>,
    windows: Query<Entity, With<PaperWindow>>,
) {
    if !open.is_changed() && !chat.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let window = commands.spawn((PaperWindow, window_node())).id();
    let line = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(INK), ChildOf(window)));
    };
    line(&mut commands, filled_in(&loc.0, &view.title));
    match view.mode {
        PaperMode::Read => line(&mut commands, plain(&filled_in(&loc.0, &view.text))),
        PaperMode::Write => {
            line(&mut commands, text("paper-write-prompt"));
            line(&mut commands, format!("{}_", chat.buffer));
            line(&mut commands, text("paper-write-keys"));
        }
        PaperMode::Label => {
            line(&mut commands, text("paper-label-prompt"));
            line(&mut commands, format!("{}_", chat.buffer));
        }
    }
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(4.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    if view.mode != PaperMode::Read {
        paper_button(&mut commands, row, text("paper-save"), PaperButton::Save);
    }
    paper_button(&mut commands, row, text("panel-close"), PaperButton::Close);
}

fn paper_button(commands: &mut Commands, parent: Entity, label: String, action: PaperButton) {
    commands
        .spawn((action, button_node(), ChildOf(parent)))
        .with_child(button_text(label))
        .observe(click_paper);
}

fn button_node() -> impl Bundle {
    (
        Node {
            border: UiRect::all(Val::Px(1.0)),
            padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.9, 0.9, 0.88)),
        BorderColor::all(EDGE),
        Hovered::default(),
    )
}

fn button_text(label: String) -> impl Bundle {
    (
        Text::new(label),
        ui_text_font(13.0),
        TextColor(Color::srgb(0.0, 0.0, 0.6)),
        Pickable::IGNORE,
    )
}

fn click_paper(
    click: On<PointerClick>,
    buttons: Query<&PaperButton>,
    mut open: ResMut<OpenPaper>,
    mut chat: ResMut<ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    match *action {
        PaperButton::Save => finish_paper(&mut open, &mut chat, &mut sender, true),
        PaperButton::Close => finish_paper(&mut open, &mut chat, &mut sender, false),
    }
}

/// Close the paper window; with `save`, send what was typed (the whole new text, or the
/// label).
pub fn finish_paper(
    open: &mut OpenPaper,
    chat: &mut ChatInput,
    sender: &mut Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    save: bool,
) {
    if save
        && let Some((target, mode)) = chat.paper
        && let Ok(mut sender) = sender.single_mut()
    {
        let typed = chat.buffer.clone();
        sender.send::<IntentChannel>(match mode {
            PaperMode::Label => ActionInput::LabelPaper { target, label: typed },
            _ => ActionInput::WritePaper { target, text: typed },
        });
    }
    chat.stop_paper();
    open.0 = None;
}

/// Rebuild the clipboard window whenever it changes (Luna's `clipboard` window: the pen,
/// then each sheet, top first, with Write, Remove, Move to top and its name to read it).
pub fn show_clipboard(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenClipboard>,
    windows: Query<Entity, With<ClipboardWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let window = commands.spawn((ClipboardWindow, window_node())).id();
    let row = |commands: &mut Commands| {
        commands
            .spawn((
                Node {
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(window),
            ))
            .id()
    };
    commands.spawn((
        Text::new(text("clipboard-title")),
        ui_text_font(14.0),
        TextColor(INK),
        ChildOf(window),
    ));
    let pen = row(&mut commands);
    let pen_label = if view.has_pen {
        "clipboard-remove-pen"
    } else {
        "clipboard-add-pen"
    };
    clip_button(
        &mut commands,
        pen,
        text(pen_label),
        ClipboardButton::Command(ClipboardCommand::Pen),
    );
    for (i, name) in view.papers.iter().enumerate() {
        let n = i as u8;
        let sheet = row(&mut commands);
        clip_button(
            &mut commands,
            sheet,
            text("clipboard-write"),
            ClipboardButton::Command(ClipboardCommand::Write(n)),
        );
        clip_button(
            &mut commands,
            sheet,
            text("clipboard-remove"),
            ClipboardButton::Command(ClipboardCommand::Remove(n)),
        );
        if i > 0 {
            clip_button(
                &mut commands,
                sheet,
                text("clipboard-top-button"),
                ClipboardButton::Command(ClipboardCommand::Top(n)),
            );
        }
        clip_button(
            &mut commands,
            sheet,
            name.clone(),
            ClipboardButton::Command(ClipboardCommand::Read(n)),
        );
    }
    let close = row(&mut commands);
    clip_button(&mut commands, close, text("panel-close"), ClipboardButton::Close);
}

fn clip_button(commands: &mut Commands, parent: Entity, label: String, action: ClipboardButton) {
    commands
        .spawn((action, button_node(), ChildOf(parent)))
        .with_child(button_text(label))
        .observe(click_clipboard);
}

fn click_clipboard(
    click: On<PointerClick>,
    buttons: Query<&ClipboardButton>,
    mut open: ResMut<OpenClipboard>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let (Ok(action), Ok(mut sender)) = (buttons.get(click.entity), sender.single_mut()) else {
        return;
    };
    match *action {
        ClipboardButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Clipboard {
                    target: view.target,
                    command,
                });
            }
        }
        ClipboardButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_is_dropped_for_reading() {
        assert_eq!(plain("[b]Hello[/b] [i]there[/i]"), "Hello there");
    }

    #[test]
    fn a_printed_sheet_is_filled_in_for_its_reader() {
        use rs13_classic::locales;
        use rs13_i18n::{Lexicon, langid};
        let lexicon = Lexicon::from_ron(locales::RU_NOUNS).unwrap();
        let ru = Localizer::new(langid!("ru"), &[locales::RU_MESSAGES], lexicon).unwrap();
        assert_eq!(
            filled_in(
                &ru,
                "[msg:id-manifest-title]
[b]Ann[/b] - [job:clown] [b"
            ),
            "Список экипажа:
[b]Ann[/b] - Клоун [b"
        );
        assert_eq!(
            filled_in(&ru, "[msg:nothing-like-this] [job:nor-this]"),
            "[msg:nothing-like-this] [job:nor-this]"
        );
    }
}

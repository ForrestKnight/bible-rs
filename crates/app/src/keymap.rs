//! Keyboard shortcuts as the design lists them (frames 26 and 29): three groups of rows, each
//! shown as keycaps. Rows that do one thing can be given another chord in Settings › Shortcuts;
//! paired rows (previous / next, the 1–6 colours…) keep theirs.
//!
//! Shortcuts are written the macOS way (⌘, ⇧…). Away from macOS, where egui's command key is
//! Ctrl, [`shown`] and [`shown_caps`] turn them into "Ctrl+Shift+P".

use std::borrow::Cow;
use std::collections::BTreeMap;

use eframe::egui::{Key, Modifiers};

use crate::actions::Action;

/// A key with modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub cmd: bool,
    pub key: Key,
}

impl Chord {
    const fn new(key: Key) -> Self {
        Self { ctrl: false, alt: false, shift: false, cmd: false, key }
    }

    const fn cmd(key: Key) -> Self {
        Self { cmd: true, ..Self::new(key) }
    }

    const fn shift_cmd(key: Key) -> Self {
        Self { shift: true, cmd: true, ..Self::new(key) }
    }

    const fn ctrl_cmd(key: Key) -> Self {
        Self { ctrl: true, cmd: true, ..Self::new(key) }
    }

    pub fn modifiers(&self) -> Modifiers {
        let mut m = Modifiers::NONE;
        if self.ctrl {
            m |= Modifiers::CTRL;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.cmd {
            m |= Modifiers::COMMAND;
        }
        m
    }

    /// With a command or control key it works while typing in a field.
    pub fn is_command(&self) -> bool {
        self.cmd || self.ctrl
    }

    fn modifier_count(&self) -> usize {
        [self.ctrl, self.alt, self.shift, self.cmd].iter().filter(|b| **b).count()
    }

    /// Keycaps in the macOS order: ⌃ ⌥ ⇧ ⌘, then the key. Elsewhere Ctrl, Alt, Shift, then the
    /// key (⌘ is Ctrl there).
    pub fn keycaps(&self) -> Vec<String> {
        let mut caps = Vec::new();
        let modifiers = if cfg!(target_os = "macos") {
            [(self.ctrl, "⌃"), (self.alt, "⌥"), (self.shift, "⇧"), (self.cmd, "⌘")]
        } else {
            [(self.ctrl || self.cmd, "Ctrl"), (self.alt, "Alt"), (self.shift, "Shift"), (false, "")]
        };
        for (on, glyph) in modifiers {
            if on {
                caps.push(glyph.to_string());
            }
        }
        caps.push(key_glyph(self.key));
        caps
    }

    /// "ctrl+cmd+F", as stored in the settings.
    pub fn spec(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for (on, name) in [(self.ctrl, "ctrl"), (self.alt, "alt"), (self.shift, "shift"), (self.cmd, "cmd")] {
            if on {
                parts.push(name);
            }
        }
        parts.push(self.key.name());
        parts.join("+")
    }

    pub fn parse(spec: &str) -> Option<Chord> {
        let mut c = Chord::new(Key::A);
        let mut key = None;
        for part in spec.split('+') {
            match part {
                "ctrl" => c.ctrl = true,
                "alt" => c.alt = true,
                "shift" => c.shift = true,
                "cmd" => c.cmd = true,
                k => key = Key::from_name(k),
            }
        }
        c.key = key?;
        Some(c)
    }

    /// A chord from a key press, if it's one a shortcut can use.
    pub fn from_press(key: Key, m: Modifiers) -> Option<Chord> {
        if matches!(key, Key::Escape | Key::Tab | Key::Enter | Key::Backspace | Key::Delete) {
            return None;
        }
        // Away from macOS the command key is Ctrl itself, so a chord holds it once.
        let ctrl = m.ctrl && cfg!(target_os = "macos");
        Some(Chord { ctrl, alt: m.alt, shift: m.shift, cmd: m.command || m.mac_cmd, key })
    }
}

fn key_glyph(k: Key) -> String {
    match k {
        Key::ArrowLeft => "←".into(),
        Key::ArrowRight => "→".into(),
        Key::ArrowUp => "↑".into(),
        Key::ArrowDown => "↓".into(),
        Key::Plus | Key::Equals => "+".into(),
        Key::Minus => "−".into(),
        Key::OpenBracket => "[".into(),
        Key::CloseBracket => "]".into(),
        Key::Comma => ",".into(),
        Key::Period => ".".into(),
        Key::Slash => "/".into(),
        Key::Space => "Space".into(),
        k => k.symbol_or_name().to_uppercase(),
    }
}

/// One row of the list.
pub struct Shortcut {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    /// The default chord of a row that can be changed; `None` for a fixed (paired) row.
    pub default: Option<Chord>,
    /// A fixed row's keycaps.
    pub fixed: &'static [&'static str],
}

const fn bindable(id: &'static str, group: &'static str, label: &'static str, chord: Chord) -> Shortcut {
    Shortcut { id, group, label, default: Some(chord), fixed: &[] }
}

const fn fixed(
    id: &'static str,
    group: &'static str,
    label: &'static str,
    caps: &'static [&'static str],
) -> Shortcut {
    Shortcut { id, group, label, default: None, fixed: caps }
}

pub const GROUPS: [&str; 3] = ["Navigate", "Verses", "View"];

/// Focus mode: ⌃⌘F, as in other Mac apps. Elsewhere F11, the usual full-screen key: ⌃ and ⌘ are
/// the same Ctrl key there, so ⌃⌘F would be Search's Ctrl+F.
const FOCUS: Chord = if cfg!(target_os = "macos") { Chord::ctrl_cmd(Key::F) } else { Chord::new(Key::F11) };

pub const ALL: [Shortcut; 18] = [
    bindable("jump", "Navigate", "Jump to a reference", Chord::cmd(Key::K)),
    fixed("chapter", "Navigate", "Previous / next chapter", &["←", "→"]),
    fixed("history", "Navigate", "Back / forward", &["⌘", "[", "⌘", "]"]),
    bindable("search", "Navigate", "Search", Chord::cmd(Key::F)),
    bindable("sidebar", "Navigate", "Books & study sidebar", Chord::cmd(Key::B)),
    fixed("verse", "Verses", "Next / previous verse", &["J", "K"]),
    fixed("highlight", "Verses", "Highlight colour · clear", &["1–6", "0"]),
    bindable("note", "Verses", "Note", Chord::new(Key::N)),
    bindable("bookmark", "Verses", "Bookmark", Chord::new(Key::B)),
    bindable("copy", "Verses", "Copy with reference", Chord::shift_cmd(Key::C)),
    bindable("compare", "Verses", "Compare translations", Chord::new(Key::C)),
    fixed("translation", "View", "Switch translation", &["⌘", "1…9"]),
    bindable("parallel", "View", "Parallel translations", Chord::shift_cmd(Key::P)),
    fixed("size", "View", "Text size", &["⌘", "+", "⌘", "−"]),
    bindable("mode", "View", "Paragraphs / verse per line", Chord::cmd(Key::L)),
    bindable("red", "View", "Red letters", Chord::cmd(Key::R)),
    bindable("theme", "View", "Theme", Chord::shift_cmd(Key::L)),
    bindable("focus", "View", "Focus mode", FOCUS),
];

/// What a changeable row does.
pub fn action(id: &str) -> Option<Action> {
    Some(match id {
        "jump" => Action::OpenPalette,
        "search" => Action::OpenSearch(None),
        "sidebar" => Action::ToggleSidebar,
        "note" => Action::EditSelectedNote,
        "bookmark" => Action::ToggleBookmark,
        "copy" => Action::Copy,
        "compare" => Action::Compare,
        "parallel" => Action::ToggleParallel,
        "mode" => Action::ToggleMode,
        "red" => Action::ToggleRedLetters,
        "theme" => Action::CycleTheme,
        "focus" => Action::ToggleFocus,
        _ => return None,
    })
}

/// A row's chord: the one chosen in Settings, else its default.
pub fn chord(s: &Shortcut, overrides: &BTreeMap<String, String>) -> Option<Chord> {
    match overrides.get(s.id) {
        Some(spec) => Chord::parse(spec),
        None => s.default,
    }
}

/// A row's keycaps as shown.
pub fn keycaps(s: &Shortcut, overrides: &BTreeMap<String, String>) -> Vec<String> {
    match s.default {
        Some(_) => chord(s, overrides).map(|c| c.keycaps()).unwrap_or_default(),
        None => shown_caps(s.fixed),
    }
}

/// Keycaps joined into one label ("⇧⌘P"; "Ctrl+Shift+P" away from macOS).
pub fn caps_label(caps: &[String]) -> String {
    if cfg!(target_os = "macos") { caps.concat() } else { caps.join("+") }
}

/// A modifier glyph's name away from macOS, and its place in the order Ctrl, Alt, Shift.
fn modifier(c: char) -> Option<(usize, &'static str)> {
    match c {
        '⌃' | '⌘' => Some((0, "Ctrl")),
        '⌥' => Some((1, "Alt")),
        '⇧' => Some((2, "Shift")),
        _ => None,
    }
}

const MODIFIER_NAMES: [&str; 3] = ["Ctrl", "Alt", "Shift"];

/// Shortcut text written the macOS way ("⇧⌘P", "Search (⌘F)", "⌘↩") as this platform shows it:
/// unchanged on macOS, "Ctrl+Shift+P", "Search (Ctrl+F)" and "Ctrl+Enter" elsewhere.
pub fn shown(text: &str) -> Cow<'_, str> {
    if cfg!(target_os = "macos") || !text.chars().any(|c| modifier(c).is_some()) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut held = [false; 3];
    let names = |held: &[bool; 3]| {
        MODIFIER_NAMES.iter().zip(held).filter(|(_, h)| **h).map(|(n, _)| *n).collect::<Vec<_>>().join("+")
    };
    for c in text.chars() {
        if let Some((i, _)) = modifier(c) {
            held[i] = true;
            continue;
        }
        if held.contains(&true) {
            out.push_str(&names(&held));
            held = [false; 3];
            // A lone modifier ("⌘ then a key") stays a word; otherwise the key follows.
            if !(c.is_whitespace() || c == ')') {
                out.push('+');
            }
        }
        match c {
            '↩' | '↵' if out.ends_with('+') => out.push_str("Enter"),
            c => out.push(c),
        }
    }
    if held.contains(&true) {
        out.push_str(&names(&held));
    }
    Cow::Owned(out)
}

/// A row of keycaps written the macOS way, as this platform shows them: unchanged on macOS;
/// elsewhere each run of modifiers in the order Ctrl, Alt, Shift (["⇧", "⌘", "P"] is
/// ["Ctrl", "Shift", "P"]).
pub fn shown_caps<S: AsRef<str>>(caps: &[S]) -> Vec<String> {
    if cfg!(target_os = "macos") {
        return caps.iter().map(|c| c.as_ref().to_string()).collect();
    }
    let mut out = Vec::with_capacity(caps.len());
    let mut held = [false; 3];
    let flush = |held: &mut [bool; 3], out: &mut Vec<String>| {
        for (name, h) in MODIFIER_NAMES.iter().zip(held.iter_mut()) {
            if std::mem::take(h) {
                out.push(name.to_string());
            }
        }
    };
    for cap in caps {
        let cap = cap.as_ref();
        let mut chars = cap.chars();
        match (chars.next().and_then(modifier), chars.next()) {
            (Some((i, _)), None) => held[i] = true,
            _ => {
                flush(&mut held, &mut out);
                out.push(shown(cap).into_owned());
            }
        }
    }
    flush(&mut held, &mut out);
    out
}

/// The changeable rows' chords and actions, those with more modifiers first (egui matches
/// shift loosely, so ⌘L would otherwise also take ⌘⇧L).
pub fn bindings(overrides: &BTreeMap<String, String>) -> Vec<(Chord, Action)> {
    let mut out: Vec<(Chord, Action)> =
        ALL.iter().filter_map(|s| Some((chord(s, overrides)?, action(s.id)?))).collect();
    out.sort_by_key(|(c, _)| std::cmp::Reverse(c.modifier_count()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_round_trip_and_read_as_keycaps() {
        let c = Chord::ctrl_cmd(Key::F);
        assert_eq!(c.spec(), "ctrl+cmd+F");
        assert_eq!(Chord::parse(&c.spec()), Some(c));
        let mut overrides = BTreeMap::new();
        overrides.insert("note".to_string(), "shift+N".to_string());
        let note = ALL.iter().find(|s| s.id == "note").unwrap();
        if cfg!(target_os = "macos") {
            assert_eq!(c.keycaps(), ["⌃", "⌘", "F"]);
            assert_eq!(Chord::shift_cmd(Key::L).keycaps(), ["⇧", "⌘", "L"]);
            assert_eq!(keycaps(note, &overrides), ["⇧", "N"]);
        } else {
            assert_eq!(c.keycaps(), ["Ctrl", "F"]);
            assert_eq!(Chord::shift_cmd(Key::L).keycaps(), ["Ctrl", "Shift", "L"]);
            assert_eq!(keycaps(note, &overrides), ["Shift", "N"]);
        }
        // More modifiers first.
        let b = bindings(&BTreeMap::new());
        assert!(b.first().unwrap().0.modifier_count() >= b.last().unwrap().0.modifier_count());
    }

    #[test]
    fn mac_shortcut_text_reads_as_this_platform_writes_it() {
        let cases = [
            ("Search (⌘F)", "Search (Ctrl+F)"),
            ("Parallel translations (⌘⇧P)", "Parallel translations (Ctrl+Shift+P)"),
            ("⇧⌘C", "Ctrl+Shift+C"),
            ("Translation (⌘1…⌘9)", "Translation (Ctrl+1…Ctrl+9)"),
            ("Done (⌘↩)", "Done (Ctrl+Enter)"),
            ("Bigger (⌘+)", "Bigger (Ctrl++)"),
            ("⌘", "Ctrl"),
            ("⇧", "Shift"),
            ("↵", "↵"),
            ("press B (or ⌘D).", "press B (or Ctrl+D)."),
            ("No shortcut here", "No shortcut here"),
        ];
        for (mac, other) in cases {
            let want = if cfg!(target_os = "macos") { mac } else { other };
            assert_eq!(shown(mac), want, "{mac}");
        }
        let caps = shown_caps(&["⇧", "⌘", "P"]);
        let label = caps_label(&caps);
        if cfg!(target_os = "macos") {
            assert_eq!(caps, ["⇧", "⌘", "P"]);
            assert_eq!(label, "⇧⌘P");
        } else {
            assert_eq!(caps, ["Ctrl", "Shift", "P"]);
            assert_eq!(label, "Ctrl+Shift+P");
            assert_eq!(shown_caps(&["⌘", "[", "⌘", "]"]), ["Ctrl", "[", "Ctrl", "]"]);
        }
    }

    #[test]
    fn focus_mode_does_not_take_search() {
        let b = bindings(&BTreeMap::new());
        let focus = b.iter().find(|(_, a)| *a == Action::ToggleFocus).unwrap().0;
        let search = b.iter().find(|(_, a)| *a == Action::OpenSearch(None)).unwrap().0;
        // Away from macOS ⌃ and ⌘ are both Ctrl, so there the chords must differ in more than that.
        let mac = cfg!(target_os = "macos");
        let as_pressed = |c: Chord| (c.key, c.alt, c.shift, c.ctrl || (c.cmd && !mac), c.cmd && mac);
        assert_ne!(as_pressed(focus), as_pressed(search));
        if !cfg!(target_os = "macos") {
            assert_eq!(focus.keycaps(), ["F11"]);
            // A Ctrl+F press reads as ⌘F, the same chord as Search, not ⌃⌘F.
            let pressed = Chord::from_press(Key::F, Modifiers::CTRL | Modifiers::COMMAND).unwrap();
            assert_eq!(pressed, search);
        }
    }
}

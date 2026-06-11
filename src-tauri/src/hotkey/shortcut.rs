use std::time::Duration;

use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const DEFAULT_HOTKEY_ID: &str = "dictation-toggle";
pub const DEFAULT_HOTKEY_SETTING: &str = "Alt+Space";
/// Short press threshold: below = toggle tap, at/above on release = push-to-talk stop.
pub const PTT_RELEASE_THRESHOLD: Duration = Duration::from_millis(400);

/// Default dictation hotkey: Alt + Space (toggle / push-to-talk hybrid).
pub fn default_shortcut() -> Shortcut {
    parse_shortcut(DEFAULT_HOTKEY_SETTING).expect("default hotkey must parse")
}

pub fn shortcut_label_from_setting(value: &str) -> String {
    value.replace("Space", "Espace")
}

pub fn shortcut_label() -> &'static str {
    "Alt+Espace"
}

/// Returns true when a held push-to-talk session should stop on key release.
pub fn should_stop_ptt_on_release(was_idle_on_press: bool, press_duration: Duration) -> bool {
    was_idle_on_press && press_duration >= PTT_RELEASE_THRESHOLD
}

/// Short tap from idle while the model is still loading — start when load completes.
pub fn is_toggle_tap(was_idle_on_press: bool, press_duration: Duration) -> bool {
    was_idle_on_press && press_duration < PTT_RELEASE_THRESHOLD
}

/// Whether a deferred model load should start recording once Whisper is ready.
pub fn should_start_after_deferred_load(still_holding: bool, deferred_toggle_intent: bool) -> bool {
    still_holding || deferred_toggle_intent
}

#[derive(Debug, thiserror::Error)]
pub enum ShortcutParseError {
    #[error("empty shortcut")]
    Empty,
    #[error("unknown modifier: {0}")]
    UnknownModifier(String),
    #[error("unknown key: {0}")]
    UnknownKey(String),
    #[error("shortcut requires at least one modifier")]
    MissingModifier,
}

pub fn parse_shortcut(value: &str) -> Result<Shortcut, ShortcutParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ShortcutParseError::Empty);
    }

    let parts: Vec<&str> = trimmed
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(ShortcutParseError::Empty);
    }

    let key_part = parts[parts.len() - 1];
    let modifier_parts = &parts[..parts.len() - 1];

    if modifier_parts.is_empty() {
        return Err(ShortcutParseError::MissingModifier);
    }

    let mut modifiers = Modifiers::empty();
    for part in modifier_parts {
        let modifier = match part.to_ascii_lowercase().as_str() {
            "alt" | "option" => Modifiers::ALT,
            "ctrl" | "control" => Modifiers::CONTROL,
            "shift" => Modifiers::SHIFT,
            "super" | "win" | "meta" | "cmd" | "command" => Modifiers::SUPER,
            other => return Err(ShortcutParseError::UnknownModifier(other.into())),
        };
        modifiers |= modifier;
    }

    let code = parse_key_code(key_part)?;
    Ok(Shortcut::new(Some(modifiers), code))
}

pub fn format_shortcut(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    let mods = shortcut.mods;
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(Modifiers::SUPER) {
        parts.push("Super");
    }
    parts.push(code_label(shortcut.key));
    parts.join("+")
}

fn code_label(code: Code) -> &'static str {
    match code {
        Code::Space => "Space",
        Code::Enter => "Enter",
        Code::Tab => "Tab",
        Code::Backspace => "Backspace",
        Code::Escape => "Escape",
        Code::F1 => "F1",
        Code::F2 => "F2",
        Code::F3 => "F3",
        Code::F4 => "F4",
        Code::F5 => "F5",
        Code::F6 => "F6",
        Code::F7 => "F7",
        Code::F8 => "F8",
        Code::F9 => "F9",
        Code::F10 => "F10",
        Code::F11 => "F11",
        Code::F12 => "F12",
        Code::KeyA => "A",
        Code::KeyB => "B",
        Code::KeyC => "C",
        Code::KeyD => "D",
        Code::KeyE => "E",
        Code::KeyF => "F",
        Code::KeyG => "G",
        Code::KeyH => "H",
        Code::KeyI => "I",
        Code::KeyJ => "J",
        Code::KeyK => "K",
        Code::KeyL => "L",
        Code::KeyM => "M",
        Code::KeyN => "N",
        Code::KeyO => "O",
        Code::KeyP => "P",
        Code::KeyQ => "Q",
        Code::KeyR => "R",
        Code::KeyS => "S",
        Code::KeyT => "T",
        Code::KeyU => "U",
        Code::KeyV => "V",
        Code::KeyW => "W",
        Code::KeyX => "X",
        Code::KeyY => "Y",
        Code::KeyZ => "Z",
        Code::Digit0 => "0",
        Code::Digit1 => "1",
        Code::Digit2 => "2",
        Code::Digit3 => "3",
        Code::Digit4 => "4",
        Code::Digit5 => "5",
        Code::Digit6 => "6",
        Code::Digit7 => "7",
        Code::Digit8 => "8",
        Code::Digit9 => "9",
        _ => "Key",
    }
}

fn parse_key_code(value: &str) -> Result<Code, ShortcutParseError> {
    let key = value.trim();
    let upper = key.to_ascii_uppercase();
    Ok(match upper.as_str() {
        "SPACE" | "ESPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "TAB" => Code::Tab,
        "BACKSPACE" => Code::Backspace,
        "ESCAPE" | "ESC" => Code::Escape,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        other if other.len() == 1 => match other.chars().next().unwrap() {
            'A'..='Z' => letter_code(other.chars().next().unwrap()),
            _ => return Err(ShortcutParseError::UnknownKey(key.into())),
        },
        _ => return Err(ShortcutParseError::UnknownKey(key.into())),
    })
}

fn letter_code(ch: char) -> Code {
    match ch {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => Code::KeyA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_can_be_constructed() {
        let shortcut = default_shortcut();
        assert!(format_shortcut(&shortcut).contains("Space"));
    }

    #[test]
    fn parses_alt_space() {
        let shortcut = parse_shortcut("Alt+Space").unwrap();
        assert_eq!(format_shortcut(&shortcut), "Alt+Space");
    }

    #[test]
    fn rejects_bare_key_without_modifier() {
        assert!(parse_shortcut("Space").is_err());
    }

    #[test]
    fn short_release_after_idle_keeps_recording() {
        assert!(!should_stop_ptt_on_release(
            true,
            Duration::from_millis(100)
        ));
    }

    #[test]
    fn short_release_while_recording_does_not_stop_ptt() {
        assert!(!should_stop_ptt_on_release(
            false,
            Duration::from_millis(100)
        ));
    }

    #[test]
    fn long_release_after_idle_stops_push_to_talk() {
        assert!(should_stop_ptt_on_release(true, Duration::from_millis(500)));
    }

    #[test]
    fn toggle_tap_is_short_press_from_idle() {
        assert!(is_toggle_tap(true, Duration::from_millis(100)));
        assert!(!is_toggle_tap(true, Duration::from_millis(500)));
        assert!(!is_toggle_tap(false, Duration::from_millis(100)));
    }

    #[test]
    fn deferred_load_starts_on_hold_or_toggle_tap() {
        assert!(should_start_after_deferred_load(true, false));
        assert!(should_start_after_deferred_load(false, true));
        assert!(!should_start_after_deferred_load(false, false));
    }
}

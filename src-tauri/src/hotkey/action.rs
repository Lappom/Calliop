use std::time::{Duration, Instant};

use crate::pipeline::PipelineState;

/// Hotkey event from the OS (pressed or released).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed,
    Released,
}

/// Mutable press-tracking state shared with the hotkey handler.
#[derive(Debug, Clone, Default)]
pub struct HotkeyPressSnapshot {
    pub press_start: Option<Instant>,
    pub was_idle_on_press: bool,
    pub shortcut_down: bool,
    pub deferred_start_pending: bool,
    pub deferred_toggle_intent: bool,
    pub busy_cancel_on_release: bool,
}

/// Context available when deciding how to react to a hotkey event.
#[derive(Debug, Clone)]
pub struct HotkeyDecisionContext {
    pub pipeline_state: PipelineState,
    pub whisper_live: bool,
    pub dictation_blocked: bool,
    pub mic_probe_active: bool,
    pub hotkey_suspended: bool,
}

/// Action the hotkey handler should execute (or ignore).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyAction {
    Ignore,
    /// Reset press tracking without pipeline side-effects (e.g. release while suspended).
    ResetPressState,
    StartRecording,
    StopRecording,
    CancelTranscribing,
    EmitBusy {
        cancelable: bool,
    },
    NotifyBlocked,
    /// Capture toggle intent during deferred model load (release only).
    CaptureDeferredToggleIntent {
        duration: Duration,
    },
}

/// Pure decision function for dictation hotkey handling.
pub fn decide_action(
    ctx: &HotkeyDecisionContext,
    press: &HotkeyPressSnapshot,
    event: HotkeyEvent,
) -> HotkeyAction {
    if ctx.mic_probe_active {
        return HotkeyAction::Ignore;
    }

    match event {
        HotkeyEvent::Pressed => {
            if ctx.hotkey_suspended {
                return HotkeyAction::Ignore;
            }
            if press.shortcut_down {
                return HotkeyAction::Ignore;
            }
            if ctx.dictation_blocked && ctx.pipeline_state == PipelineState::Idle {
                return HotkeyAction::NotifyBlocked;
            }
            match ctx.pipeline_state {
                PipelineState::Idle => HotkeyAction::StartRecording,
                PipelineState::Recording => HotkeyAction::StopRecording,
                PipelineState::Transcribing => HotkeyAction::EmitBusy { cancelable: true },
                PipelineState::Injecting => HotkeyAction::EmitBusy { cancelable: false },
            }
        }
        HotkeyEvent::Released => {
            if ctx.hotkey_suspended {
                if press.shortcut_down {
                    return HotkeyAction::ResetPressState;
                }
                return HotkeyAction::Ignore;
            }
            if !press.shortcut_down {
                return HotkeyAction::Ignore;
            }
            let duration = press
                .press_start
                .map(|start| start.elapsed())
                .unwrap_or_default();

            if press.busy_cancel_on_release && ctx.pipeline_state == PipelineState::Transcribing {
                return HotkeyAction::CancelTranscribing;
            }

            if press.deferred_start_pending {
                return HotkeyAction::CaptureDeferredToggleIntent { duration };
            }

            if ctx.pipeline_state == PipelineState::Recording
                && super::should_stop_ptt_on_release(press.was_idle_on_press, duration)
            {
                return HotkeyAction::StopRecording;
            }

            HotkeyAction::Ignore
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::{is_toggle_tap, should_stop_ptt_on_release};

    fn ctx(pipeline: PipelineState) -> HotkeyDecisionContext {
        HotkeyDecisionContext {
            pipeline_state: pipeline,
            whisper_live: true,
            dictation_blocked: false,
            mic_probe_active: false,
            hotkey_suspended: false,
        }
    }

    #[test]
    fn press_from_idle_starts_recording() {
        let action = decide_action(
            &ctx(PipelineState::Idle),
            &HotkeyPressSnapshot::default(),
            HotkeyEvent::Pressed,
        );
        assert_eq!(action, HotkeyAction::StartRecording);
    }

    #[test]
    fn press_while_recording_stops() {
        let action = decide_action(
            &ctx(PipelineState::Recording),
            &HotkeyPressSnapshot::default(),
            HotkeyEvent::Pressed,
        );
        assert_eq!(action, HotkeyAction::StopRecording);
    }

    #[test]
    fn release_while_suspended_resets_press_state() {
        let mut c = ctx(PipelineState::Idle);
        c.hotkey_suspended = true;
        let press = HotkeyPressSnapshot {
            shortcut_down: true,
            press_start: Some(Instant::now()),
            ..Default::default()
        };
        let action = decide_action(&c, &press, HotkeyEvent::Released);
        assert_eq!(action, HotkeyAction::ResetPressState);
    }

    #[test]
    fn long_release_from_idle_stops_ptt() {
        let press = HotkeyPressSnapshot {
            shortcut_down: true,
            was_idle_on_press: true,
            press_start: Some(Instant::now() - Duration::from_millis(500)),
            ..Default::default()
        };
        let action = decide_action(
            &ctx(PipelineState::Recording),
            &press,
            HotkeyEvent::Released,
        );
        assert_eq!(action, HotkeyAction::StopRecording);
        assert!(should_stop_ptt_on_release(true, Duration::from_millis(500)));
    }

    #[test]
    fn short_release_from_idle_does_not_stop() {
        let press = HotkeyPressSnapshot {
            shortcut_down: true,
            was_idle_on_press: true,
            press_start: Some(Instant::now() - Duration::from_millis(100)),
            ..Default::default()
        };
        let action = decide_action(
            &ctx(PipelineState::Recording),
            &press,
            HotkeyEvent::Released,
        );
        assert_eq!(action, HotkeyAction::Ignore);
        assert!(is_toggle_tap(true, Duration::from_millis(100)));
    }

    #[test]
    fn release_during_transcribing_cancels_when_flagged() {
        let press = HotkeyPressSnapshot {
            shortcut_down: true,
            busy_cancel_on_release: true,
            ..Default::default()
        };
        let action = decide_action(
            &ctx(PipelineState::Transcribing),
            &press,
            HotkeyEvent::Released,
        );
        assert_eq!(action, HotkeyAction::CancelTranscribing);
    }
}

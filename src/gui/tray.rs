//! Windows system tray recording indicator.
//!
//! Uses `tray-icon` (Shell_NotifyIcon) to show a coloured tray icon and a
//! right-click context menu reflecting the daemon state. Runs its own Win32
//! message pump on a dedicated thread.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG,
    MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT, WM_QUIT,
};

use crate::cli::signals::DaemonSignal;
use crate::domain::daemon::{DaemonState, StateUpdate};

const ICON_SIZE: u32 = 32;
/// Time to block on `MsgWaitForMultipleObjectsEx` between iterations.
/// Short enough that state updates and tooltip refreshes feel instant.
const POLL_TIMEOUT_MS: u32 = 200;
/// Minimum interval between tooltip refreshes during recording.
const TOOLTIP_REFRESH_MS: u64 = 500;

#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("Failed to create tray icon: {0}")]
    TrayInit(String),
    #[error("Failed to build icon bitmap: {0}")]
    IconBuild(String),
}

/// Run the Windows tray indicator.
///
/// Blocks the calling thread on a Win32 message pump. Returns `Ok(())` on a
/// clean shutdown (e.g. the user picked "Quit daemon" from the tray menu).
/// Returns `Err` if the tray icon could not be created (e.g. running under a
/// non-interactive session) — the caller should warn and continue headless.
pub fn run_indicator(
    state_rx: broadcast::Receiver<StateUpdate>,
    signal_tx: tokio::sync::mpsc::Sender<DaemonSignal>,
) -> Result<(), TrayError> {
    // Bridge the async broadcast channel into a sync mpsc the message loop can
    // drain without an async context (same pattern as `layer_shell::run_indicator`).
    let (state_sync_tx, state_sync_rx) = mpsc::channel::<StateUpdate>();
    std::thread::spawn(move || {
        let mut rx = state_rx;
        while let Ok(update) = rx.blocking_recv() {
            if state_sync_tx.send(update).is_err() {
                break;
            }
        }
    });

    // Cache the two visible-state icons once.
    let recording_icon = make_icon([220, 50, 50, 255])?;
    let processing_icon = make_icon([255, 180, 50, 255])?;

    // Build the context menu. Item IDs are matched in the event loop.
    let menu = Menu::new();
    let toggle = MenuItem::with_id(MenuId::new("toggle"), "Toggle Recording", true, None);
    let cancel = MenuItem::with_id(MenuId::new("cancel"), "Cancel", false, None);
    let separator = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id(MenuId::new("quit"), "Quit daemon", true, None);
    menu.append_items(&[&toggle, &cancel, &separator, &quit])
        .map_err(|e| TrayError::TrayInit(e.to_string()))?;

    let mut tray: Option<TrayIcon> = None;
    let mut current_state = DaemonState::Idle;
    let mut current_elapsed_ms: u64 = 0;
    let mut last_tooltip_update = Instant::now();
    let menu_event_rx = MenuEvent::receiver();

    loop {
        // 1. Drain pending state updates (non-blocking).
        let mut state_changed = false;
        while let Ok(update) = state_sync_rx.try_recv() {
            if update.state != current_state {
                state_changed = true;
            }
            current_state = update.state;
            current_elapsed_ms = update.elapsed_ms;
        }

        // 2. Apply state changes to the tray.
        if state_changed {
            apply_state(
                &mut tray,
                current_state,
                current_elapsed_ms,
                &recording_icon,
                &processing_icon,
                &menu,
                &toggle,
                &cancel,
            )?;
            last_tooltip_update = Instant::now();
        } else if current_state == DaemonState::Recording
            && last_tooltip_update.elapsed() >= Duration::from_millis(TOOLTIP_REFRESH_MS)
        {
            if let Some(t) = tray.as_ref() {
                let _ = t.set_tooltip(Some(format_tooltip(current_state, current_elapsed_ms)));
            }
            last_tooltip_update = Instant::now();
        }

        // 3. Drain menu events and forward them as daemon signals.
        while let Ok(event) = menu_event_rx.try_recv() {
            let signal = match event.id.0.as_str() {
                "toggle" => Some(DaemonSignal::Toggle),
                "cancel" => Some(DaemonSignal::Cancel),
                "quit" => Some(DaemonSignal::Shutdown),
                _ => None,
            };
            if let Some(sig) = signal {
                let _ = signal_tx.blocking_send(sig);
                if sig == DaemonSignal::Shutdown {
                    return Ok(());
                }
            }
        }

        // 4. Pump native Win32 messages.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Wait up to POLL_TIMEOUT_MS for new input or a tray/menu event,
            // so we don't busy-loop. The wait returns on any new Win32
            // message OR after the timeout, which lets us re-check state.
            MsgWaitForMultipleObjectsEx(
                0,
                std::ptr::null(),
                POLL_TIMEOUT_MS,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_state(
    tray: &mut Option<TrayIcon>,
    state: DaemonState,
    elapsed_ms: u64,
    recording_icon: &Icon,
    processing_icon: &Icon,
    menu: &Menu,
    toggle: &MenuItem,
    cancel: &MenuItem,
) -> Result<(), TrayError> {
    match state {
        DaemonState::Idle => {
            // Drop the tray icon entirely → vanishes from the notification area.
            *tray = None;
        }
        DaemonState::Recording => {
            toggle.set_text("Stop Recording");
            toggle.set_enabled(true);
            cancel.set_enabled(true);
            let t = ensure_tray(tray, recording_icon, menu)?;
            let _ = t.set_icon(Some(recording_icon.clone()));
            let _ = t.set_tooltip(Some(format_tooltip(state, elapsed_ms)));
        }
        DaemonState::Processing => {
            toggle.set_text("Toggle Recording");
            toggle.set_enabled(false);
            cancel.set_enabled(false);
            let t = ensure_tray(tray, processing_icon, menu)?;
            let _ = t.set_icon(Some(processing_icon.clone()));
            let _ = t.set_tooltip(Some(format_tooltip(state, elapsed_ms)));
        }
    }
    Ok(())
}

fn ensure_tray<'a>(
    slot: &'a mut Option<TrayIcon>,
    initial_icon: &Icon,
    menu: &Menu,
) -> Result<&'a TrayIcon, TrayError> {
    if slot.is_none() {
        let built = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_icon(initial_icon.clone())
            .with_tooltip("SmartScribe")
            .build()
            .map_err(|e| TrayError::TrayInit(e.to_string()))?;
        *slot = Some(built);
    }
    Ok(slot.as_ref().unwrap())
}

fn format_tooltip(state: DaemonState, elapsed_ms: u64) -> String {
    match state {
        DaemonState::Recording => {
            let total_secs = elapsed_ms / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("SmartScribe — Recording {}:{:02}", mins, secs)
        }
        DaemonState::Processing => "SmartScribe — Processing…".to_string(),
        DaemonState::Idle => "SmartScribe".to_string(),
    }
}

/// Build a 32x32 RGBA icon with an antialiased filled circle in the given colour.
fn make_icon(rgba: [u8; 4]) -> Result<Icon, TrayError> {
    let size = ICON_SIZE as i32;
    let mut buf = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    let r = (size as f32 / 2.0) - 2.5; // ~13px radius for a 32px icon
    let r_outer = r + 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            // Antialiased coverage in the [r-1, r+1] band.
            let coverage = if dist <= r - 0.5 {
                1.0
            } else if dist >= r_outer {
                0.0
            } else {
                (r_outer - dist).clamp(0.0, 1.0)
            };
            if coverage > 0.0 {
                let idx = ((y * size + x) * 4) as usize;
                buf[idx] = rgba[0];
                buf[idx + 1] = rgba[1];
                buf[idx + 2] = rgba[2];
                buf[idx + 3] = (rgba[3] as f32 * coverage) as u8;
            }
        }
    }

    Icon::from_rgba(buf, ICON_SIZE, ICON_SIZE).map_err(|e| TrayError::IconBuild(e.to_string()))
}

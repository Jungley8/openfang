//! Color palette matching the OpenFang landing page design system.
//!
//! Core palette from globals.css + code syntax from constants.ts.

#![allow(dead_code)] // Full palette — some colors reserved for future screens.

use ratatui::style::{Color, Modifier, Style};

// ── Core Palette (from landing page globals.css) ────────────────────────────

pub const ACCENT: Color = Color::Rgb(255, 92, 0); // #FF5C00 — OpenFang orange
pub const ACCENT_DIM: Color = Color::Rgb(224, 82, 0); // #E05200

pub const BG_PRIMARY: Color = Color::Rgb(237, 236, 235); // #EDECEB — light mode
pub const BG_CARD: Color = Color::Rgb(255, 255, 255); // #FFFFFF — white surface
pub const BG_HOVER: Color = Color::Rgb(240, 238, 236); // #F0EEEC
pub const BG_CODE: Color = Color::Rgb(232, 230, 227); // #E8E6E3

pub const TEXT_PRIMARY: Color = Color::Rgb(26, 24, 23); // #1A1817 — dark text on light bg
pub const TEXT_SECONDARY: Color = Color::Rgb(61, 57, 53); // #3D3935 — WCAG AA pass
pub const TEXT_TERTIARY: Color = Color::Rgb(107, 101, 96); // #6B6560 — WCAG AA pass

pub const BORDER: Color = Color::Rgb(213, 210, 207); // #D5D2CF — light border

// ── Semantic Colors (darker variants for light background contrast) ─────────

pub const GREEN: Color = Color::Rgb(22, 163, 74); // #16A34A — success
pub const BLUE: Color = Color::Rgb(37, 99, 235); // #2563EB — info
pub const YELLOW: Color = Color::Rgb(217, 119, 6); // #D97706 — warning
pub const RED: Color = Color::Rgb(220, 38, 38); // #DC2626 — error
pub const PURPLE: Color = Color::Rgb(147, 51, 234); // #9333EA — decorators

// ── Backward-compat aliases ─────────────────────────────────────────────────

pub const CYAN: Color = BLUE;
pub const DIM: Color = TEXT_SECONDARY;

// ── Reusable styles ─────────────────────────────────────────────────────────

pub fn title_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn selected_style() -> Style {
    Style::default().fg(ACCENT).bg(BG_HOVER)
}

pub fn dim_style() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn input_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn hint_style() -> Style {
    Style::default().fg(TEXT_TERTIARY)
}

// ── Tab bar styles ──────────────────────────────────────────────────────────

pub fn tab_active() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn tab_inactive() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

// ── State badge styles ──────────────────────────────────────────────────────

pub fn badge_running() -> Style {
    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
}

pub fn badge_created() -> Style {
    Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
}

pub fn badge_suspended() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

pub fn badge_terminated() -> Style {
    Style::default().fg(TEXT_TERTIARY)
}

pub fn badge_crashed() -> Style {
    Style::default().fg(RED).add_modifier(Modifier::BOLD)
}

/// Return badge text + style for an agent state string.
pub fn state_badge(state: &str) -> (&'static str, Style) {
    let lower = state.to_lowercase();
    if lower.contains("run") {
        ("[RUN]", badge_running())
    } else if lower.contains("creat") || lower.contains("new") || lower.contains("idle") {
        ("[NEW]", badge_created())
    } else if lower.contains("sus") || lower.contains("paus") {
        ("[SUS]", badge_suspended())
    } else if lower.contains("term") || lower.contains("stop") || lower.contains("end") {
        ("[END]", badge_terminated())
    } else if lower.contains("err") || lower.contains("crash") || lower.contains("fail") {
        ("[ERR]", badge_crashed())
    } else {
        ("[---]", dim_style())
    }
}

// ── Table / channel styles ──────────────────────────────────────────────────

pub fn table_header() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn channel_ready() -> Style {
    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
}

pub fn channel_missing() -> Style {
    Style::default().fg(YELLOW)
}

pub fn channel_off() -> Style {
    dim_style()
}

// ── Spinner ─────────────────────────────────────────────────────────────────

pub const SPINNER_FRAMES: &[&str] = &[
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];

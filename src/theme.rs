//! Studio themes — port of `THEMES` from `src/constants/shortcuts.ts`.

use crate::model::{StudioTheme, ThemeId};
use egui::Color32;

/// Parse "#rrggbb" / "#rgb" into a Color32. Falls back to magenta on bad input.
pub fn hex32(s: &str) -> Color32 {
    let s = s.trim_start_matches('#');
    let (r, g, b) = match s.len() {
        6 => (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ),
        3 => {
            let d = |c: &str| u8::from_str_radix(&c.repeat(2), 16);
            (d(&s[0..1]), d(&s[1..2]), d(&s[2..3]))
        }
        _ => return Color32::MAGENTA,
    };
    match (r, g, b) {
        (Ok(r), Ok(g), Ok(b)) => Color32::from_rgb(r, g, b),
        _ => Color32::MAGENTA,
    }
}

pub const THEMES: &[StudioTheme] = &[
    StudioTheme {
        id: ThemeId::DarkCc,
        name: "Studio Dark Pro",
        is_light: false,
        bg_main: "#1e1e1e",
        bg_panel: "#252526",
        bg_canvas: "#18181a",
        bg_subtle: "#141416",
        border: "#333333",
        border_strong: "#444444",
        text_primary: "#ededed",
        text_secondary: "#cbd5e1",
        text_muted: "#94a3b8",
        chip_bg: "#000000",
        input_bg: "#000000",
        hover_bg: "#3a3a3a",
        accent: "#0d99ff",
    },
    StudioTheme {
        id: ThemeId::Charcoal,
        name: "Charcoal Studio",
        is_light: false,
        bg_main: "#18181b",
        bg_panel: "#27272a",
        bg_canvas: "#09090b",
        bg_subtle: "#121214",
        border: "#3f3f46",
        border_strong: "#52525b",
        text_primary: "#fafafa",
        text_secondary: "#d4d4d8",
        text_muted: "#a1a1aa",
        chip_bg: "#000000",
        input_bg: "#000000",
        hover_bg: "#3f3f46",
        accent: "#38bdf8",
    },
    StudioTheme {
        id: ThemeId::Midnight,
        name: "Midnight Deep Blue",
        is_light: false,
        bg_main: "#0f172a",
        bg_panel: "#1e293b",
        bg_canvas: "#020617",
        bg_subtle: "#090d16",
        border: "#334155",
        border_strong: "#475569",
        text_primary: "#f8fafc",
        text_secondary: "#cbd5e1",
        text_muted: "#94a3b8",
        chip_bg: "#000000",
        input_bg: "#000000",
        hover_bg: "#334155",
        accent: "#60a5fa",
    },
    StudioTheme {
        id: ThemeId::LightStudio,
        name: "Light Accessible Pro (WCAG AA)",
        is_light: true,
        bg_main: "#f1f5f9",
        bg_panel: "#ffffff",
        bg_canvas: "#cbd5e1",
        bg_subtle: "#f8fafc",
        border: "#cbd5e1",
        border_strong: "#64748b",
        text_primary: "#0f172a",
        text_secondary: "#1e293b",
        text_muted: "#475569",
        chip_bg: "#e2e8f0",
        input_bg: "#ffffff",
        hover_bg: "#e2e8f0",
        accent: "#0284c7",
    },
    StudioTheme {
        id: ThemeId::LightContrast,
        name: "Light High-Contrast (WCAG AAA)",
        is_light: true,
        bg_main: "#ffffff",
        bg_panel: "#ffffff",
        bg_canvas: "#94a3b8",
        bg_subtle: "#f1f5f9",
        border: "#475569",
        border_strong: "#0f172a",
        text_primary: "#000000",
        text_secondary: "#0f172a",
        text_muted: "#1e293b",
        chip_bg: "#cbd5e1",
        input_bg: "#ffffff",
        hover_bg: "#cbd5e1",
        accent: "#0369a1",
    },
];

pub fn theme_by_id(id: ThemeId) -> &'static StudioTheme {
    THEMES.iter().find(|t| t.id == id).unwrap_or(&THEMES[0])
}

/// Apply a StudioTheme to the egui visuals so native widgets follow suit.
pub fn apply_theme(ctx: &egui::Context, theme: &StudioTheme) {
    let mut style = (*ctx.style()).clone();
    style.visuals = if theme.is_light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    let v = &mut style.visuals;
    v.panel_fill = hex32(theme.bg_panel);
    v.window_fill = hex32(theme.bg_panel);
    v.extreme_bg_color = hex32(theme.bg_subtle);
    v.faint_bg_color = hex32(theme.bg_subtle);
    v.widgets.noninteractive.bg_fill = hex32(theme.bg_panel);
    v.widgets.noninteractive.fg_stroke.color = hex32(theme.text_primary);
    v.widgets.inactive.bg_fill = hex32(theme.bg_subtle);
    v.widgets.inactive.fg_stroke.color = hex32(theme.text_secondary);
    v.widgets.hovered.bg_fill = hex32(theme.hover_bg);
    v.widgets.hovered.fg_stroke.color = hex32(theme.text_primary);
    v.widgets.active.bg_fill = hex32(theme.accent);
    v.widgets.active.fg_stroke.color = Color32::WHITE;
    v.selection.bg_fill = hex32(theme.accent);
    v.hyperlink_color = hex32(theme.accent);
    ctx.set_style(style);
}

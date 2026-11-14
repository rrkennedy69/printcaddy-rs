//! Palettes from the TypeScript app. Light is the everyday look. X-Ray is the visor.

use egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    Dark,
    Light,
    Graphite,
    Night,
    Thermal,
    Xray,
    Blocks,
    Forge,
    Works,
}

impl ThemeId {
    pub const ALL: [ThemeId; 9] = [
        ThemeId::Dark,
        ThemeId::Light,
        ThemeId::Graphite,
        ThemeId::Night,
        ThemeId::Thermal,
        ThemeId::Xray,
        ThemeId::Blocks,
        ThemeId::Forge,
        ThemeId::Works,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Dark => "Dark",
            ThemeId::Light => "Light",
            ThemeId::Graphite => "Graphite",
            ThemeId::Night => "Night Vision",
            ThemeId::Thermal => "Thermal",
            ThemeId::Xray => "X-Ray",
            ThemeId::Blocks => "Blocks",
            ThemeId::Forge => "Forge",
            ThemeId::Works => "Works",
        }
    }

    pub fn from_label(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.label() == text)
    }
}

pub struct Palette {
    pub panel: Color32,
    pub text: Color32,
    pub viewport: [f32; 4],
    pub plate: [f32; 3],
    pub grid_minor: [f32; 3],
    pub grid_major: [f32; 3],
    pub grid_border: [f32; 3],
    pub light: bool,
    pub xray: bool,
}

pub fn palette(theme: ThemeId) -> Palette {
    match theme {
        ThemeId::Dark => Palette {
            panel: Color32::from_rgb(28, 35, 41),
            text: Color32::from_rgb(213, 221, 227),
            viewport: [0.11, 0.145, 0.17, 1.0],
            plate: [0.16, 0.20, 0.24],
            grid_minor: [0.23, 0.31, 0.36],
            grid_major: [0.42, 0.53, 0.60],
            grid_border: [0.62, 0.72, 0.78],
            light: false,
            xray: false,
        },
        ThemeId::Light => Palette {
            panel: Color32::from_rgb(244, 241, 234),
            text: Color32::from_rgb(28, 36, 48),
            viewport: [0.965, 0.953, 0.925, 1.0],
            plate: [0.80, 0.81, 0.82],
            grid_minor: [0.42, 0.44, 0.46],
            grid_major: [0.28, 0.30, 0.33],
            grid_border: [0.16, 0.18, 0.20],
            light: true,
            xray: false,
        },
        ThemeId::Graphite => Palette {
            panel: Color32::from_rgb(42, 45, 49),
            text: Color32::from_rgb(230, 230, 230),
            viewport: [0.10, 0.11, 0.12, 1.0],
            plate: [0.22, 0.23, 0.24],
            grid_minor: [0.28, 0.29, 0.31],
            grid_major: [0.48, 0.50, 0.52],
            grid_border: [0.70, 0.72, 0.74],
            light: false,
            xray: false,
        },
        ThemeId::Night => Palette {
            panel: Color32::from_rgb(20, 0, 0),
            text: Color32::from_rgb(255, 70, 60),
            viewport: [0.03, 0.0, 0.0, 1.0],
            plate: [0.14, 0.02, 0.02],
            grid_minor: [0.35, 0.05, 0.04],
            grid_major: [0.75, 0.12, 0.08],
            grid_border: [1.0, 0.25, 0.18],
            light: false,
            xray: false,
        },
        ThemeId::Thermal => Palette {
            panel: Color32::from_rgb(26, 18, 8),
            text: Color32::from_rgb(255, 176, 32),
            viewport: [0.07, 0.05, 0.02, 1.0],
            plate: [0.20, 0.12, 0.04],
            grid_minor: [0.40, 0.22, 0.05],
            grid_major: [0.90, 0.45, 0.08],
            grid_border: [1.0, 0.72, 0.20],
            light: false,
            xray: false,
        },
        ThemeId::Xray => Palette {
            panel: Color32::from_rgb(11, 8, 32),
            text: Color32::from_rgb(230, 224, 255),
            viewport: [0.04, 0.03, 0.10, 1.0],
            plate: [0.10, 0.08, 0.22],
            grid_minor: [0.23, 0.18, 0.43],
            grid_major: [0.55, 0.48, 0.95],
            grid_border: [0.90, 0.88, 1.0],
            light: false,
            xray: true,
        },
        ThemeId::Blocks => Palette {
            panel: Color32::from_rgb(247, 244, 239),
            text: Color32::from_rgb(44, 58, 71),
            viewport: [0.91, 0.93, 0.95, 1.0],
            plate: [0.82, 0.86, 0.90],
            grid_minor: [0.70, 0.78, 0.84],
            grid_major: [0.35, 0.55, 0.72],
            grid_border: [0.15, 0.35, 0.55],
            light: true,
            xray: false,
        },
        ThemeId::Forge => Palette {
            panel: Color32::from_rgb(43, 43, 43),
            text: Color32::from_rgb(232, 232, 232),
            viewport: [0.16, 0.16, 0.16, 1.0],
            plate: [0.28, 0.28, 0.28],
            grid_minor: [0.30, 0.30, 0.30],
            grid_major: [0.72, 0.42, 0.12],
            grid_border: [0.90, 0.55, 0.15],
            light: false,
            xray: false,
        },
        ThemeId::Works => Palette {
            panel: Color32::from_rgb(243, 243, 243),
            text: Color32::from_rgb(26, 26, 26),
            viewport: [0.75, 0.80, 0.84, 1.0],
            plate: [0.68, 0.72, 0.76],
            grid_minor: [0.55, 0.62, 0.68],
            grid_major: [0.15, 0.35, 0.62],
            grid_border: [0.08, 0.20, 0.40],
            light: true,
            xray: false,
        },
    }
}

pub fn apply(ctx: &egui::Context, theme: ThemeId, mobile: bool) {
    let colors = palette(theme);
    let mut visuals = if colors.light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    visuals.panel_fill = colors.panel;
    visuals.window_fill = colors.panel;
    visuals.extreme_bg_color = colors.panel;
    visuals.override_text_color = Some(colors.text);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        let pad = if mobile { 10.0 } else { 6.0 };
        let y = if mobile { 8.0 } else { 3.0 };
        style.spacing.button_padding = egui::vec2(pad, y);
        style.spacing.item_spacing = egui::vec2(8.0, if mobile { 6.0 } else { 4.0 });
        if mobile {
            style.interaction.interact_radius = 10.0;
        }
    });
}

pub fn swatches() -> [u32; 8] {
    [
        0x8A98A6, 0xE26D5A, 0xE6A23C, 0x7DCEA0, 0x5DADE2, 0xAF7AC5, 0xF4F1EA, 0x2C3E50,
    ]
}

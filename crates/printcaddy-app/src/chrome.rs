//! The live editor's header, icon ribbon, left rail, and bottom bar.

use egui::{Color32, Rect, Sense, TextureHandle, Ui, Vec2};

use crate::icons::{ink, IconSet};

pub const PAGE: Color32 = Color32::from_rgb(247, 244, 236);
pub const INK: Color32 = Color32::from_rgb(55, 46, 36);
pub const MUTED: Color32 = Color32::from_rgb(140, 122, 100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Undo,
    Redo,
    Duplicate,
    Delete,
    Shapes,
    Group,
    Ungroup,
    Intersect,
    CutOut,
    Fillet,
    Slice,
    Layers,
    Import,
    Export,
    AddPlate,
    Settings,
    Shortcuts,
    SnapToggle,
    SketchTab,
    EditorTab,
    HomeCamera,
    Fit,
    ZoomIn,
    ZoomOut,
    ToggleRulers,
    ToggleGrid,
    Guide,
    Bug,
    NotBuilt,
    SetInches(bool),
    SetSnap(bool),
}

struct Tool {
    icon: &'static str,
    label: &'static str,
    cmd: Cmd,
    built: bool,
}

struct Group {
    name: &'static str,
    tools: &'static [Tool],
}

const HOME: &[Tool] = &[
    Tool { icon: "home-dashboard", label: "Home", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "undo", label: "Undo", cmd: Cmd::Undo, built: true },
    Tool { icon: "redo", label: "Redo", cmd: Cmd::Redo, built: true },
    Tool { icon: "history", label: "History", cmd: Cmd::NotBuilt, built: false },
];
const CLIPBOARD: &[Tool] = &[
    Tool { icon: "copy", label: "Copy", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "paste", label: "Paste", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "duplicate", label: "Duplicate", cmd: Cmd::Duplicate, built: true },
    Tool { icon: "delete", label: "Delete", cmd: Cmd::Delete, built: true },
];
const SHAPES: &[Tool] = &[
    Tool { icon: "add-shape", label: "Add shape", cmd: Cmd::Shapes, built: true },
    Tool { icon: "show-object-list", label: "Object list", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "hide-selected", label: "Hide selected", cmd: Cmd::NotBuilt, built: false },
];
const COMBINE: &[Tool] = &[
    Tool { icon: "group", label: "Group", cmd: Cmd::Group, built: true },
    Tool { icon: "bundle", label: "Bundle", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "ungroup", label: "Ungroup", cmd: Cmd::Ungroup, built: true },
    Tool { icon: "boolean-intersection", label: "Intersect", cmd: Cmd::Intersect, built: true },
    Tool { icon: "cut-out", label: "Cut out", cmd: Cmd::CutOut, built: true },
    Tool { icon: "hull", label: "Hull", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "loft-sketches", label: "Loft sketches", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "show-common-part", label: "Common part", cmd: Cmd::NotBuilt, built: false },
];
const ARRANGE: &[Tool] = &[
    Tool { icon: "align", label: "Align", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "snap-to-grid", label: "Snap to grid", cmd: Cmd::SnapToggle, built: true },
    Tool { icon: "drop-to-workplane", label: "Drop", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "lay-flat-on-face", label: "Lay flat", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "align-faces", label: "Align faces", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "center-on-workplane", label: "Center", cmd: Cmd::NotBuilt, built: false },
];
const MODIFY: &[Tool] = &[
    Tool { icon: "mirror", label: "Mirror", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "set-rotation-pivot", label: "Pivot", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "pattern", label: "Pattern", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "hole-pattern", label: "Hole pattern", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "face", label: "Face", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "split", label: "Split", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "flexi-joints", label: "Flexi joints", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "cookie-cutter", label: "Cookie cutter", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "chamfer", label: "Chamfer", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "fillet", label: "Fillet", cmd: Cmd::Fillet, built: true },
    Tool { icon: "hollow", label: "Hollow", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "offset", label: "Offset", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "delete-face", label: "Delete face", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "simplify-mesh", label: "Simplify", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "surface-texture", label: "Texture", cmd: Cmd::NotBuilt, built: false },
];
const MANAGE: &[Tool] = &[
    Tool { icon: "note", label: "Note", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "import", label: "Import", cmd: Cmd::Import, built: true },
    Tool { icon: "export", label: "Export", cmd: Cmd::Export, built: true },
    Tool { icon: "attachments", label: "Attachments", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "slice", label: "Slice", cmd: Cmd::Slice, built: true },
    Tool { icon: "layer-view", label: "Layers", cmd: Cmd::Layers, built: true },
    Tool { icon: "send-to-bambu-studio", label: "To Bambu", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "ai-helper", label: "AI", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "collision", label: "Collision", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "add-plate", label: "Add plate", cmd: Cmd::AddPlate, built: true },
    Tool { icon: "full-screen", label: "Full screen", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "gravity", label: "Gravity", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "workspace-settings", label: "Settings", cmd: Cmd::Settings, built: true },
];
const HELP: &[Tool] = &[
    Tool { icon: "search-commands-ctrl-k", label: "Search", cmd: Cmd::NotBuilt, built: false },
    Tool { icon: "quick-guide", label: "Guide", cmd: Cmd::Guide, built: false },
    Tool { icon: "keyboard-shortcuts", label: "Shortcuts", cmd: Cmd::Shortcuts, built: true },
    Tool { icon: "interactive-help", label: "Help", cmd: Cmd::NotBuilt, built: false },
];

const ROW1: &[Group] = &[
    Group { name: "HOME", tools: HOME },
    Group { name: "CLIPBOARD", tools: CLIPBOARD },
    Group { name: "SHAPES", tools: SHAPES },
    Group { name: "COMBINE", tools: COMBINE },
    Group { name: "ARRANGE", tools: ARRANGE },
];
const ROW2: &[Group] = &[
    Group { name: "MODIFY", tools: MODIFY },
    Group { name: "MANAGE", tools: MANAGE },
    Group { name: "HELP", tools: HELP },
];

pub struct HeaderState<'a> {
    pub name: &'a mut String,
    pub sketching: bool,
    pub lang: &'a str,
}

pub fn header(ui: &mut Ui, icons: &IconSet, state: &mut HeaderState<'_>) -> Option<Cmd> {
    let mut cmd = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
        ui.add(egui::Image::new(&icons.logo).fit_to_exact_size(Vec2::splat(28.0)));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("PrintCaddy").size(16.0).strong().color(INK));
            ui.label(egui::RichText::new("Easy 3D CAD for 3D printing").size(11.0).color(MUTED));
        });
        ui.add_space(18.0);
        if tab(ui, "3D EDITOR", !state.sketching) {
            cmd = Some(Cmd::EditorTab);
        }
        if tab(ui, "SKETCH", state.sketching) {
            cmd = Some(Cmd::SketchTab);
        }
        ui.add_space(8.0);
        let edit = egui::TextEdit::singleline(state.name)
            .desired_width(220.0)
            .hint_text("Untitled design");
        ui.add(edit);
        ui.label(egui::RichText::new("✎").color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(state.lang.to_ascii_uppercase()).size(12.0).color(INK));
            if let Some(tex) = icons.toolbar.get("language") {
                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(14.0)).tint(INK));
            }
            ui.add_space(10.0);
            ui.label(egui::RichText::new("System").size(12.0).color(INK));
            if let Some(tex) = icons.toolbar.get("theme") {
                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(14.0)).tint(INK));
            }
        });
    });
    cmd
}

fn tab(ui: &mut Ui, label: &str, on: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(92.0, 28.0), Sense::click());
    let fill = if on {
        Color32::WHITE
    } else {
        Color32::from_rgba_premultiplied(0, 0, 0, 0)
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    if on {
        ui.painter().rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0_f32, Color32::from_rgb(226, 214, 196)),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        if on { INK } else { MUTED },
    );
    response.clicked()
}

fn ribbon(ui: &mut Ui, icons: &IconSet, row: &[Group], can: &dyn Fn(Cmd) -> bool) -> Option<Cmd> {
    let mut cmd = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        for (index, group) in row.iter().enumerate() {
            if index > 0 {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 52.0), Sense::hover());
                ui.painter().line_segment(
                    [
                        egui::pos2(rect.center().x, rect.top() + 10.0),
                        egui::pos2(rect.center().x, rect.bottom() - 4.0),
                    ],
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(226, 216, 200)),
                );
            }
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(group.name)
                        .size(9.0)
                        .color(MUTED)
                        .strong(),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
                    for tool in group.tools {
                        let enabled = tool.built && can(tool.cmd);
                        let clicked = icon_tool(ui, icons.toolbar.get(tool.icon), tool.label, enabled);
                        if clicked && enabled {
                            cmd = Some(tool.cmd);
                        } else if clicked && !tool.built {
                            cmd = Some(Cmd::NotBuilt);
                        }
                    }
                });
            });
        }
    });
    cmd
}

pub fn ribbon_all(ui: &mut Ui, icons: &IconSet, can: &dyn Fn(Cmd) -> bool) -> Option<Cmd> {
    let first = ribbon(ui, icons, ROW1, can);
    let second = ribbon(ui, icons, ROW2, can);
    second.or(first)
}

fn rail_button(ui: &mut Ui, tex: Option<&TextureHandle>, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    if response.hovered() && enabled {
        ui.painter().rect_filled(rect, 8.0, Color32::from_rgb(244, 236, 222));
    }
    if let Some(tex) = tex {
        let icon = Rect::from_center_size(rect.center(), Vec2::splat(18.0));
        ui.painter().image(
            tex.id(),
            icon,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            ink(enabled),
        );
    }
    response.clicked()
}

fn icon_tool(ui: &mut Ui, tex: Option<&TextureHandle>, label: &str, enabled: bool) -> bool {
    let width = (label.len() as f32 * 4.6_f32 + 8.0).clamp(36.0, 58.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 44.0), Sense::click());
    if response.hovered() && enabled {
        ui.painter().rect_filled(rect, 6.0, Color32::from_rgb(255, 252, 246));
    }
    let tint = ink(enabled);
    if let Some(tex) = tex {
        let icon = Rect::from_center_size(rect.center() - Vec2::new(0.0, 7.0), Vec2::splat(20.0));
        ui.painter().image(
            tex.id(),
            icon,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 7.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(9.0),
        tint,
    );
    response.clicked()
}

const RAIL: &[(&str, Cmd, bool)] = &[
    ("show-labels", Cmd::NotBuilt, false),
    ("home", Cmd::HomeCamera, true),
    ("center-view-on-selection", Cmd::Fit, true),
    ("zoom-in", Cmd::ZoomIn, true),
    ("zoom-out", Cmd::ZoomOut, true),
    ("orthographic-view", Cmd::NotBuilt, false),
    ("open-a-second-view", Cmd::NotBuilt, false),
    ("rulers", Cmd::ToggleRulers, true),
    ("grid", Cmd::ToggleGrid, true),
    ("double-click-to-zoom", Cmd::NotBuilt, false),
    ("hide-workplane", Cmd::NotBuilt, false),
    ("place-workplane", Cmd::NotBuilt, false),
    ("tape-measure", Cmd::NotBuilt, false),
    ("place-framing-square", Cmd::NotBuilt, false),
    ("section-view", Cmd::NotBuilt, false),
];

pub fn rail(ui: &mut Ui, icons: &IconSet) -> Option<Cmd> {
    let mut cmd = None;
    egui::Frame::NONE
        .fill(Color32::from_rgb(255, 252, 247))
        .corner_radius(18.0)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(230, 220, 206)))
        .inner_margin(egui::Margin::symmetric(4, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
            for (icon, action, built) in RAIL {
                let enabled = *built;
                if rail_button(ui, icons.rail.get(*icon), enabled) {
                    cmd = Some(if enabled { *action } else { Cmd::NotBuilt });
                }
            }
        });
    cmd
}

pub fn footer(ui: &mut Ui) -> Option<Cmd> {
    let mut cmd = None;
    ui.horizontal(|ui| {
        if link(ui, "User guide") {
            cmd = Some(Cmd::Guide);
        }
        ui.add_space(12.0);
        if link(ui, "Bug report") {
            cmd = Some(Cmd::Bug);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("printcaddy-rs").size(11.0).color(MUTED));
            ui.label(egui::RichText::new("·").color(MUTED));
            ui.label(
                egui::RichText::new("Source code & license (MIT)")
                    .size(11.0)
                    .color(MUTED),
            );
        });
    });
    cmd
}

fn link(ui: &mut Ui, label: &str) -> bool {
    let response = ui.add(egui::Label::new(egui::RichText::new(label).size(12.0).color(INK)).sense(Sense::click()));
    response.clicked()
}

pub fn snap_bar(ui: &mut Ui, snap: bool, step: f32, inches: bool) -> Option<Cmd> {
    let mut cmd = None;
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(247, 244, 236, 230))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Snap Grid").size(12.0).color(INK));
                let label = if snap { format!("{step:.1} mm") } else { "Off".into() };
                ui.menu_button(label, |ui| {
                    if ui.button("Off").clicked() {
                        cmd = Some(Cmd::SetSnap(false));
                    }
                    if ui.button("1.0 mm").clicked() {
                        cmd = Some(Cmd::SetSnap(true));
                    }
                });
                if unit_chip(ui, "mm", !inches) {
                    cmd = Some(Cmd::SetInches(false));
                }
                if unit_chip(ui, "in", inches) {
                    cmd = Some(Cmd::SetInches(true));
                }
            });
        });
    let _ = step;
    cmd
}

fn unit_chip(ui: &mut Ui, label: &str, on: bool) -> bool {
    let fill = if on {
        Color32::from_rgb(90, 122, 168)
    } else {
        Color32::from_rgb(232, 226, 216)
    };
    let text = if on { Color32::WHITE } else { INK };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(28.0, 20.0), Sense::click());
    ui.painter().rect_filled(rect, 5.0, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        text,
    );
    response.clicked()
}

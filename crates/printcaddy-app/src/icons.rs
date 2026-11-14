//! Icons copied from the live TypeScript editor, keyed to white so a tint sets the ink.

use std::collections::HashMap;

use egui::{Color32, ColorImage, Context, TextureHandle};

use crate::icon_bytes;

pub struct IconSet {
    pub toolbar: HashMap<&'static str, TextureHandle>,
    pub rail: HashMap<&'static str, TextureHandle>,
    pub shapes: HashMap<&'static str, TextureHandle>,
    pub logo: TextureHandle,
}

impl IconSet {
    pub fn load(ctx: &Context) -> Self {
        let toolbar_names = [
            "add-plate", "add-shape", "ai-helper", "align", "align-faces", "attachments",
            "boolean-intersection", "bundle", "center-on-workplane", "chamfer", "collision",
            "cookie-cutter", "copy", "cut-out", "delete", "delete-face", "drop-to-workplane",
            "duplicate", "export", "face", "fillet", "flexi-joints", "full-screen", "gravity",
            "group", "hide-selected", "history", "hole-pattern", "hollow", "home-dashboard",
            "hull", "import", "interactive-help", "keyboard-shortcuts", "language",
            "lay-flat-on-face", "layer-view", "loft-sketches", "mirror", "note", "offset",
            "paste", "pattern", "quick-guide", "redo", "search-commands-ctrl-k",
            "send-to-bambu-studio", "set-rotation-pivot", "show-common-part",
            "show-object-list", "simplify-mesh", "slice", "snap-to-grid", "split",
            "surface-texture", "theme", "undo", "ungroup", "visibility-options",
            "workspace-settings",
        ];
        let rail_names = [
            "back-view", "bottom-view", "center-view-on-selection", "double-click-to-zoom",
            "front-view", "grid", "hide-workplane", "home", "open-a-second-view",
            "orthographic-view", "place-framing-square", "place-workplane", "rulers",
            "section-view", "show-labels", "tape-measure", "top-view", "zoom-in", "zoom-out",
        ];
        let shape_names = [
            "bentTube", "box", "cone", "counterbore", "countersink", "crescent", "cylinder",
            "doubleCountersink", "dovetail", "ellipse", "gridfinity", "half-sphere", "heart",
            "hinge", "honeycomb", "iconTag", "knurl", "loft", "nameTag", "paraboloid",
            "polygon", "pyramid", "qrCode", "round-roof", "roundedBox", "ruler", "slot",
            "sphere", "spring", "star", "teardrop", "text", "thread", "torus", "tube", "vase",
            "wedge",
        ];
        Self {
            toolbar: load_map(ctx, "toolbar", &toolbar_names, icon_bytes::toolbar),
            rail: load_map(ctx, "rail", &rail_names, icon_bytes::rail),
            shapes: load_map(ctx, "shape", &shape_names, icon_bytes::shape),
            logo: ctx.load_texture(
                "logo",
                decode_color(icon_bytes::logo()),
                egui::TextureOptions::LINEAR,
            ),
        }
    }
}

fn load_map(
    ctx: &Context,
    prefix: &str,
    names: &[&'static str],
    bytes_of: fn(&str) -> Option<&'static [u8]>,
) -> HashMap<&'static str, TextureHandle> {
    let mut map = HashMap::new();
    for name in names {
        let Some(bytes) = bytes_of(name) else {
            continue;
        };
        let image = decode(bytes);
        let handle = ctx.load_texture(
            format!("{prefix}-{name}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        map.insert(*name, handle);
    }
    map
}

fn decode_color(bytes: &[u8]) -> ColorImage {
    let image = image::load_from_memory(bytes)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1))
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, image.as_raw())
}

fn decode(bytes: &[u8]) -> ColorImage {
    let image = image::load_from_memory(bytes)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1))
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let mut pixels = image.into_raw();
    for chunk in pixels.chunks_exact_mut(4) {
        if chunk[3] == 0 {
            continue;
        }
        chunk[0] = 255;
        chunk[1] = 255;
        chunk[2] = 255;
    }
    ColorImage::from_rgba_unmultiplied(size, &pixels)
}

pub fn ink(enabled: bool) -> Color32 {
    if enabled {
        Color32::from_rgb(62, 52, 42)
    } else {
        Color32::from_rgb(196, 186, 174)
    }
}

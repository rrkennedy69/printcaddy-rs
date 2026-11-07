use std::collections::HashMap;

use egui::{Color32, Pos2, Rect, Sense};
use printcaddy_core::{
    document_to_stl, import_stl, ray_plane_y, snap_mm, Document, History, ObjectId, Shape,
    DEFAULT_MODEL_COLOR, DEFAULT_SNAP_MM, PLATE_SIZE_MM,
};

use crate::camera::Camera;
use crate::files::{self, Incoming};
use crate::renderer::ViewportPainter;
use crate::scene::{self, UploadedMesh};

const DRAG_SLOP: f32 = 5.0;

enum Gesture {
    None,
    Orbit {
        last: Pos2,
        origin: Pos2,
        engaged: bool,
    },
    Move {
        id: ObjectId,
        plane_y: f32,
        grab_x: f32,
        grab_z: f32,
        start_x: f32,
        start_z: f32,
        origin: Pos2,
        engaged: bool,
    },
    Pinch {
        dist: f32,
        mid: Pos2,
    },
}

pub struct PrintCaddyApp {
    doc: Document,
    camera: Camera,
    painter: ViewportPainter,
    meshes: HashMap<(u64, u64), std::sync::Arc<UploadedMesh>>,
    incoming: Incoming,
    history: History,
    snap: bool,
    notice: String,
    gesture: Gesture,
    touches: HashMap<u64, Pos2>,
    /// The frame a touch ends, the browser also synthesises a mouse click. Skip that click.
    swallow_mouse: bool,
}

impl PrintCaddyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Keep the panels light. The viewport clears itself to a dark workshop colour.
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        Self {
            doc: Document::new(),
            camera: Camera::default(),
            painter: ViewportPainter::new(),
            meshes: HashMap::new(),
            incoming: files::incoming_slot(),
            history: History::default(),
            snap: true,
            notice:
                "Drag empty space to orbit. Right-drag or two fingers pan. Wheel or pinch zooms."
                    .into(),
            gesture: Gesture::None,
            touches: HashMap::new(),
            swallow_mouse: false,
        }
    }

    fn aspect_of(rect: Rect) -> f32 {
        rect.width().max(1.0) / rect.height().max(1.0)
    }

    fn ray_at(&self, rect: Rect, pos: Pos2) -> printcaddy_core::Ray {
        let aspect = Self::aspect_of(rect);
        scene::screen_ray(rect, pos, self.camera.view_proj(aspect))
    }

    fn status_text(&self) -> String {
        match self.doc.selection().and_then(|id| self.doc.object(id)) {
            Some(object) => {
                let (w, d, h) = object.size();
                format!(
                    "objects={} selected={} x={:.2} y={:.2} z={:.2} rx={:.1} ry={:.1} rz={:.1} w={:.2} d={:.2} h={:.2} color={}",
                    self.doc.objects().len(),
                    object.name,
                    object.position.x,
                    object.position.y,
                    object.position.z,
                    object.rotation.x,
                    object.rotation.y,
                    object.rotation.z,
                    w,
                    d,
                    h,
                    object.color.hex_string()
                )
            }
            None => format!("objects={} selected=none", self.doc.objects().len()),
        }
    }

    fn take_import(&mut self) {
        let Some((name, bytes)) = self.incoming.borrow_mut().take() else {
            return;
        };
        self.history.end();
        self.history.begin(&self.doc);
        match import_stl(&mut self.doc, &name, &bytes) {
            Ok(id) => {
                let label = self.doc.object(id).map(|o| o.name.clone()).unwrap_or(name);
                self.notice = format!("Imported {label}");
            }
            Err(_) => {
                self.history.cancel();
                self.notice = format!("Could not read {name} as STL");
            }
        }
        self.history.end();
    }

    fn export(&mut self) {
        let selected = self.doc.selection().is_some();
        let bytes = document_to_stl(&self.doc, selected);
        files::download_stl(&bytes);
        let which = if selected {
            "the selection"
        } else {
            "the plate"
        };
        self.notice = format!("Exported {which} as STL");
    }

    fn pointer_move(&mut self, rect: Rect, pos: Pos2) {
        match self.gesture {
            Gesture::Move {
                id,
                plane_y,
                grab_x,
                grab_z,
                origin,
                engaged,
                ..
            } => {
                let now = pos.distance(origin) > DRAG_SLOP || engaged;
                if !now {
                    return;
                }
                if !engaged {
                    self.history.end();
                    self.history.begin(&self.doc);
                }
                if let Gesture::Move { engaged, .. } = &mut self.gesture {
                    *engaged = true;
                }
                let ray = self.ray_at(rect, pos);
                if let Some(hit) = ray_plane_y(ray, plane_y) {
                    let step = if self.snap { DEFAULT_SNAP_MM } else { 0.0 };
                    self.doc.move_object_to(
                        id,
                        snap_mm(hit.x - grab_x, step),
                        snap_mm(hit.z - grab_z, step),
                    );
                }
            }
            Gesture::Orbit {
                last,
                origin,
                engaged,
            } => {
                let now = pos.distance(origin) > DRAG_SLOP || engaged;
                if !now {
                    return;
                }
                self.camera.orbit(pos.x - last.x, pos.y - last.y);
                if let Gesture::Orbit { last, engaged, .. } = &mut self.gesture {
                    *last = pos;
                    *engaged = true;
                }
            }
            _ => {}
        }
    }

    fn begin_primary(&mut self, rect: Rect, pos: Pos2) {
        let ray = self.ray_at(rect, pos);
        if let Some(id) = self.doc.hit_test(ray) {
            self.doc.select(Some(id));
            let object = self.doc.object(id).unwrap();
            let plane_y = object.position.y;
            let (grab_x, grab_z) = if let Some(hit) = ray_plane_y(ray, plane_y) {
                (hit.x - object.position.x, hit.z - object.position.z)
            } else {
                (0.0, 0.0)
            };
            self.gesture = Gesture::Move {
                id,
                plane_y,
                grab_x,
                grab_z,
                start_x: object.position.x,
                start_z: object.position.z,
                origin: pos,
                engaged: false,
            };
        } else {
            self.gesture = Gesture::Orbit {
                last: pos,
                origin: pos,
                engaged: false,
            };
        }
    }

    fn end_primary(&mut self) {
        match self.gesture {
            Gesture::Move {
                engaged: false, id, ..
            } => self.doc.select(Some(id)),
            Gesture::Orbit { engaged: false, .. } => self.doc.select(None),
            Gesture::Move {
                id,
                start_x,
                start_z,
                engaged: true,
                ..
            } => {
                let _ = (id, start_x, start_z);
            }
            _ => {}
        }
        self.gesture = Gesture::None;
        self.history.end();
    }

    fn handle_touch(&mut self, ctx: &egui::Context, rect: Rect) -> bool {
        let events = ctx.input(|input| input.events.clone());
        let mut saw = false;
        for event in events {
            let egui::Event::Touch { id, phase, pos, .. } = event else {
                continue;
            };
            if !rect.contains(pos)
                && !matches!(phase, egui::TouchPhase::End | egui::TouchPhase::Cancel)
                && self.touches.is_empty()
            {
                continue;
            }
            saw = true;
            let key = touch_key(id);
            match phase {
                egui::TouchPhase::Start => {
                    self.touches.insert(key, pos);
                    self.on_touch_count_changed(rect);
                }
                egui::TouchPhase::Move => {
                    self.touches.insert(key, pos);
                    self.on_touch_moved(rect, pos);
                }
                egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                    self.touches.remove(&key);
                    self.on_touch_count_changed(rect);
                }
            }
        }
        saw || !self.touches.is_empty()
    }

    fn on_touch_count_changed(&mut self, rect: Rect) {
        match self.touches.len() {
            0 => self.end_primary(),
            1 => {
                if let Gesture::Move {
                    id,
                    start_x,
                    start_z,
                    engaged: true,
                    ..
                } = self.gesture
                {
                    // A second finger takes the move back, the way the current app does.
                    self.doc.move_object_to(id, start_x, start_z);
                    self.history.cancel();
                }
                let pos = *self.touches.values().next().unwrap();
                if self.touches.len() == 1 && matches!(self.gesture, Gesture::Pinch { .. }) {
                    self.gesture = Gesture::Orbit {
                        last: pos,
                        origin: pos,
                        engaged: true,
                    };
                } else if matches!(self.gesture, Gesture::None) {
                    self.begin_primary(rect, pos);
                }
            }
            _ => {
                if let Gesture::Move {
                    id,
                    start_x,
                    start_z,
                    engaged: true,
                    ..
                } = self.gesture
                {
                    self.doc.move_object_to(id, start_x, start_z);
                    self.history.cancel();
                }
                let pts: Vec<Pos2> = self.touches.values().copied().take(2).collect();
                if pts.len() == 2 {
                    self.gesture = Gesture::Pinch {
                        dist: pts[0].distance(pts[1]).max(1.0),
                        mid: midpoint(pts[0], pts[1]),
                    };
                }
            }
        }
    }

    fn on_touch_moved(&mut self, rect: Rect, _pos: Pos2) {
        if self.touches.len() >= 2 {
            let pts: Vec<Pos2> = self.touches.values().copied().take(2).collect();
            if pts.len() < 2 {
                return;
            }
            let dist = pts[0].distance(pts[1]).max(1.0);
            let mid = midpoint(pts[0], pts[1]);
            if let Gesture::Pinch {
                dist: prev,
                mid: prev_mid,
            } = self.gesture
            {
                self.camera.zoom_factor(prev / dist);
                self.camera.pan(mid.x - prev_mid.x, mid.y - prev_mid.y);
            }
            self.gesture = Gesture::Pinch { dist, mid };
            return;
        }
        if let Some(pos) = self.touches.values().next().copied() {
            self.pointer_move(rect, pos);
        }
    }

    fn handle_mouse(&mut self, ctx: &egui::Context, response: &egui::Response, rect: Rect) {
        if self.swallow_mouse {
            self.swallow_mouse = false;
            return;
        }
        let pointer = ctx.input(|input| input.pointer.clone());
        let over = pointer.hover_pos().is_some_and(|pos| rect.contains(pos));
        if over
            && (pointer.button_down(egui::PointerButton::Secondary)
                || pointer.button_down(egui::PointerButton::Middle))
        {
            let delta = pointer.delta();
            if delta != egui::Vec2::ZERO {
                self.camera.pan(delta.x, delta.y);
            }
            self.gesture = Gesture::None;
            return;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = pointer.press_origin().or(pointer.interact_pos()) {
                self.begin_primary(rect, pos);
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(pos) = pointer.interact_pos() {
                self.pointer_move(rect, pos);
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            || (matches!(self.gesture, Gesture::Move { .. } | Gesture::Orbit { .. })
                && pointer.primary_released())
        {
            self.end_primary();
        } else if response.clicked() {
            if let Some(pos) = pointer.interact_pos() {
                self.begin_primary(rect, pos);
                self.end_primary();
            }
        }
        let scroll = ctx.input(|input| input.smooth_scroll_delta.y);
        if response.hovered() && scroll.abs() > 0.0 {
            self.camera.zoom_factor((-scroll * 0.0016).exp());
        }
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.doc.selection() else {
            ui.label("Nothing selected");
            return;
        };
        let Some(object) = self.doc.object(id).cloned() else {
            return;
        };
        ui.label(format!("Selected: {}", object.name));
        let (width0, depth0, height0) = object.size();
        let mut position = object.position;
        let mut rotation = object.rotation;
        let mut width = width0;
        let mut depth = depth0;
        let mut height = height0;
        let mut pose = false;
        let mut sized = false;
        ui.label("Position (mm)");
        pose |= number(ui, "X", &mut position.x);
        pose |= number(ui, "Y", &mut position.y);
        pose |= number(ui, "Z", &mut position.z);
        ui.label("Rotation (degrees)");
        pose |= number(ui, "Rot X", &mut rotation.x);
        pose |= number(ui, "Rot Y", &mut rotation.y);
        pose |= number(ui, "Rot Z", &mut rotation.z);
        ui.label("Size (mm)");
        sized |= number(ui, "Width", &mut width);
        sized |= number(ui, "Depth", &mut depth);
        sized |= number(ui, "Height", &mut height);
        if (pose || sized) && !self.history.is_open() {
            self.history.begin(&self.doc);
        }
        if pose {
            self.doc.set_position(id, position);
            self.doc.set_rotation(id, rotation);
        }
        if sized
            && ((width - width0).abs() > 1e-4
                || (depth - depth0).abs() > 1e-4
                || (height - height0).abs() > 1e-4)
        {
            self.doc.set_size(id, width, depth, height);
        }
    }
}

impl eframe::App for PrintCaddyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.take_import();
        let command = ctx.input(|input| input.modifiers.command);
        let undo_key =
            ctx.input(|input| command && input.key_pressed(egui::Key::Z) && !input.modifiers.shift);
        let redo_key = ctx.input(|input| {
            command
                && (input.key_pressed(egui::Key::Y)
                    || (input.modifiers.shift && input.key_pressed(egui::Key::Z)))
        });
        let delete_key = !ctx.wants_keyboard_input()
            && ctx.input(|input| {
                input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace)
            });
        if undo_key {
            self.history.undo(&mut self.doc);
            self.notice = "Undid the last change".into();
        } else if redo_key {
            self.history.redo(&mut self.doc);
            self.notice = "Redid the change".into();
        } else if delete_key {
            if let Some(id) = self.doc.selection() {
                self.history.end();
                self.history.begin(&self.doc);
                self.doc.delete(id);
                self.history.end();
                self.notice = "Deleted the selection".into();
            }
        }

        let mut export = false;
        let mut import = false;
        let mut chosen: Option<Shape> = None;
        let mut undo = false;
        let mut redo = false;
        let mut duplicate = false;
        let mut delete = false;

        egui::SidePanel::left("tools")
            .exact_width(280.0)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading("PrintCaddy");
                    ui.label("Rust and WebAssembly");
                    ui.horizontal(|ui| {
                        undo = ui.add_enabled(self.history.can_undo(), egui::Button::new("Undo")).clicked();
                        redo = ui.add_enabled(self.history.can_redo(), egui::Button::new("Redo")).clicked();
                    });
                    ui.separator();
                    ui.label("Shapes");
                    ui.horizontal_wrapped(|ui| {
                        for shape in Shape::ALL {
                            if ui.button(shape.label()).clicked() {
                                chosen = Some(shape);
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        import = ui.button("Import STL").clicked();
                        export = ui.button("Export STL").clicked();
                    });
                    ui.horizontal(|ui| {
                        duplicate = ui.button("Duplicate").clicked();
                        delete = ui.button("Delete").clicked();
                    });
                    ui.checkbox(&mut self.snap, "Snap to 1 mm grid");
                    ui.separator();
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        3.0,
                        Color32::from_rgb(DEFAULT_MODEL_COLOR.r, DEFAULT_MODEL_COLOR.g, DEFAULT_MODEL_COLOR.b),
                    );
                    ui.label(format!("Default colour {}", DEFAULT_MODEL_COLOR.hex_string()));
                    ui.label(format!("Plate {PLATE_SIZE_MM:.0} × {PLATE_SIZE_MM:.0} mm"));
                    self.properties(ui);
                    ui.separator();
                    ui.label(self.notice.clone());
                    if let Some(err) = self.painter.error() {
                        ui.colored_label(Color32::LIGHT_RED, err);
                    }
                    ui.add_space(8.0);
                    ui.small("Drag empty space to orbit, drag a shape to move it. Right or middle button pans. Wheel or pinch zooms.");
                    ui.small("Two fingers pan and pinch. A second finger puts a move back. Ctrl+Z undoes.");
                });
            });

        if let Some(shape) = chosen {
            self.history.end();
            self.history.begin(&self.doc);
            let id = self.doc.add_shape(shape);
            let name = self
                .doc
                .object(id)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| shape.label().into());
            self.history.end();
            self.notice = format!("Added {name}");
        }
        if undo {
            self.history.undo(&mut self.doc);
            self.notice = "Undid the last change".into();
        }
        if redo {
            self.history.redo(&mut self.doc);
            self.notice = "Redid the change".into();
        }
        if duplicate {
            if let Some(id) = self.doc.selection() {
                self.history.end();
                self.history.begin(&self.doc);
                self.doc.duplicate(id);
                self.history.end();
                self.notice = "Duplicated the selection".into();
            }
        }
        if delete {
            if let Some(id) = self.doc.selection() {
                self.history.end();
                self.history.begin(&self.doc);
                self.doc.delete(id);
                self.history.end();
                self.notice = "Deleted the selection".into();
            }
        }
        if import {
            files::pick_stl(self.incoming.clone());
        }
        if export {
            self.export();
        }

        let mut view_rect = Rect::NOTHING;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let available = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
                view_rect = rect;
                let touching = self.handle_touch(ctx, rect);
                if touching {
                    self.swallow_mouse = true;
                } else {
                    self.handle_mouse(ctx, &response, rect);
                }
                let aspect = Self::aspect_of(rect);
                let frame =
                    scene::draw_frame(&self.doc, self.camera.view_proj(aspect), &mut self.meshes);
                let callback = self.painter.callback(frame);
                ui.painter().add(egui::PaintCallback { rect, callback });
            });
        let _ = view_rect;
        if ctx.input(|input| input.pointer.any_released()) {
            self.history.end();
        }
        files::publish_status(&format!("{} | {}", self.status_text(), self.notice));
    }
}

fn number(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(value).speed(0.1).fixed_decimals(2))
    })
    .inner
    .changed()
}

fn midpoint(a: Pos2, b: Pos2) -> Pos2 {
    Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn touch_key(id: egui::TouchId) -> u64 {
    id.0
}

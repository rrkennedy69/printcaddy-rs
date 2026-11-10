use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use egui::{Color32, Pos2, Rect, Sense};
use printcaddy_core::{
    document_from_obj, document_from_step, document_to_obj, document_to_step, document_to_stl,
    export_3mf, import_3mf, import_dxf, import_stl, import_svg, ray_plane_y, snap_mm, BooleanOp,
    Document, History, ObjectId, Ray, Shape, SketchDef, SketchOp, DEFAULT_MODEL_COLOR,
    DEFAULT_SNAP_MM, PLATE_SIZE_MM,
};

use crate::camera::Camera;
use crate::drive;
use crate::files::{self, Incoming};
use crate::library::{self, SavedProject};
use crate::overlay::{self, Gizmo};
use crate::renderer::ViewportPainter;
use crate::scene::{self, UploadedMesh};
use crate::theme::{self, ThemeId};

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
        group: Vec<(ObjectId, f32, f32)>,
        origin: Pos2,
        engaged: bool,
    },
    Pinch {
        dist: f32,
        mid: Pos2,
    },
    Axis {
        gizmo: Gizmo,
        origin: Pos2,
        pivot: Pos2,
        axis_tip: Pos2,
        starts: Vec<(ObjectId, glam::Vec3)>,
        engaged: bool,
    },
    Turn {
        gizmo: Gizmo,
        pivot_screen: Pos2,
        pivot_world: glam::Vec3,
        origin: Pos2,
        starts: Vec<(ObjectId, glam::Vec3, glam::Vec3)>,
        engaged: bool,
    },
    Cube {
        origin: Pos2,
        engaged: bool,
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
    add_select: bool,
    sketching: bool,
    draw_path: bool,
    profile: Vec<[f32; 2]>,
    sketch_path: Vec<[f32; 2]>,
    sketch_height: f32,
    sketch_angle: f32,
    fillet_radius: f32,
    editing_sketch: Option<ObjectId>,
    pending_point: Option<[f32; 2]>,
    /// Face centres of the view cube from the last paint, for the next click.
    cube_faces: Vec<(overlay::CubeFace, Pos2)>,
    notice: String,
    theme: ThemeId,
    mobile: bool,
    show_settings: bool,
    show_projects: bool,
    show_updates: bool,
    project_name: String,
    projects: Vec<SavedProject>,
    update_text: String,
    update_newer: bool,
    update_slot: Rc<RefCell<Option<String>>>,
    drive_text: String,
    drive_note: drive::DriveNote,
    drive_file: drive::DriveFile,
    project_in: files::TextIncoming,
    show_import: bool,
    show_export: bool,
    gesture: Gesture,
    touches: HashMap<u64, Pos2>,
    /// The frame a touch ends, the browser also synthesises a mouse click. Skip that click.
    swallow_mouse: bool,
}

impl PrintCaddyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (theme_label, mobile) = library::load_settings();
        let theme = ThemeId::from_label(&theme_label).unwrap_or(ThemeId::Dark);
        theme::apply(&cc.egui_ctx, theme, mobile);
        Self {
            doc: Document::new(),
            camera: Camera::default(),
            painter: ViewportPainter::new(),
            meshes: HashMap::new(),
            incoming: files::incoming_slot(),
            history: History::default(),
            snap: true,
            add_select: false,
            sketching: false,
            draw_path: false,
            profile: Vec::new(),
            sketch_path: Vec::new(),
            sketch_height: 10.0,
            sketch_angle: 360.0,
            fillet_radius: 2.0,
            editing_sketch: None,
            pending_point: None,
            cube_faces: Vec::new(),
            notice:
                "Drag empty space to orbit. Right-drag or two fingers pan. Wheel or pinch zooms."
                    .into(),
            theme,
            mobile,
            show_settings: false,
            show_projects: false,
            show_updates: false,
            project_name: "Untitled".into(),
            projects: library::load(),
            update_text: "Check whether a newer build is published.".into(),
            update_newer: false,
            update_slot: Rc::new(RefCell::new(None)),
            drive_text: "Sign in with the same Google account flow as the TypeScript app.".into(),
            drive_note: Rc::new(RefCell::new(None)),
            drive_file: Rc::new(RefCell::new(None)),
            project_in: files::text_slot(),
            show_import: false,
            show_export: false,
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

    fn begin_sketch(&mut self) {
        self.sketching = true;
        self.draw_path = false;
        self.profile.clear();
        self.sketch_path.clear();
        self.editing_sketch = None;
        self.pending_point = None;
        self.notice = "Click the plate to draw the profile.".into();
    }

    fn edit_selected_sketch(&mut self) {
        let Some(id) = self.doc.selection() else {
            self.notice = "Select a sketch to edit.".into();
            return;
        };
        let Some(def) = self.doc.sketch(id).cloned() else {
            self.notice = "The selection is not a sketch.".into();
            return;
        };
        self.profile = def.profile;
        self.sketch_path = def.path;
        self.sketch_height = def.height;
        self.sketch_angle = def.angle_deg;
        self.editing_sketch = Some(id);
        self.sketching = true;
        self.draw_path = false;
        self.pending_point = None;
        self.notice = "Editing the sketch. Cancel leaves it unchanged.".into();
    }

    fn cancel_sketch(&mut self) {
        self.sketching = false;
        self.draw_path = false;
        self.profile.clear();
        self.sketch_path.clear();
        self.editing_sketch = None;
        self.pending_point = None;
        self.notice = "Sketch cancelled.".into();
    }

    fn finish_sketch(&mut self, op: SketchOp) {
        let def = if let Some(id) = self.editing_sketch {
            let Some(mut stored) = self.doc.sketch(id).cloned() else {
                self.notice = "That sketch is no longer in the document.".into();
                return;
            };
            stored.profile = self.profile.clone();
            stored.path = self.sketch_path.clone();
            stored.op = op;
            stored.height = self.sketch_height;
            stored.angle_deg = self.sketch_angle;
            stored
        } else {
            SketchDef::on_plate(
                op,
                self.profile.clone(),
                self.sketch_path.clone(),
                self.sketch_height,
                self.sketch_angle,
            )
        };
        self.history.end();
        self.history.begin(&self.doc);
        let result = if let Some(id) = self.editing_sketch {
            self.doc.replace_sketch(id, def).map(|()| id)
        } else {
            self.doc.add_sketch(def)
        };
        match result {
            Ok(_) => {
                self.history.end();
                let name = match op {
                    SketchOp::Extrude => "Extruded",
                    SketchOp::Revolve => "Revolved",
                    SketchOp::Sweep => "Swept",
                };
                self.notice = format!("{name} the sketch.");
                self.sketching = false;
                self.pending_point = None;
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err.to_string();
            }
        }
    }

    fn sketch_uv(&self, rect: Rect, pos: Pos2) -> Option<[f32; 2]> {
        let ray = self.ray_at(rect, pos);
        let step = if self.snap { DEFAULT_SNAP_MM } else { 0.0 };
        if let Some(id) = self.editing_sketch {
            let object = self.doc.object(id)?;
            let def = self.doc.sketch(id)?;
            let origin = object.position + glam::Vec3::from(def.origin);
            let axis_u = glam::Vec3::from(def.axis_u);
            let axis_v = glam::Vec3::from(def.axis_v);
            let hit = ray_onto_plane(ray, origin, axis_u.cross(axis_v))?;
            let delta = hit - origin;
            return Some([
                snap_mm(delta.dot(axis_u), step),
                snap_mm(delta.dot(axis_v), step),
            ]);
        }
        let hit = ray_plane_y(ray, 0.0)?;
        Some([snap_mm(hit.x, step), snap_mm(-hit.z, step)])
    }

    fn uv_world(&self, uv: [f32; 2]) -> glam::Vec3 {
        if let Some(id) = self.editing_sketch {
            if let (Some(object), Some(def)) = (self.doc.object(id), self.doc.sketch(id)) {
                return object.position
                    + glam::Vec3::from(def.origin)
                    + glam::Vec3::from(def.axis_u) * uv[0]
                    + glam::Vec3::from(def.axis_v) * uv[1];
            }
        }
        glam::Vec3::new(uv[0], 0.0, -uv[1])
    }

    fn paint_sketch(&self, painter: &egui::Painter, aspect: f32, rect: Rect) {
        if !self.sketching {
            return;
        }
        self.paint_chain(
            painter,
            aspect,
            rect,
            &self.profile,
            Color32::from_rgb(255, 196, 64),
            true,
        );
        self.paint_chain(
            painter,
            aspect,
            rect,
            &self.sketch_path,
            Color32::from_rgb(80, 200, 255),
            false,
        );
    }

    fn paint_chain(
        &self,
        painter: &egui::Painter,
        aspect: f32,
        rect: Rect,
        points: &[[f32; 2]],
        color: Color32,
        close: bool,
    ) {
        let screen: Vec<Pos2> = points
            .iter()
            .filter_map(|uv| overlay::project(&self.camera, aspect, rect, self.uv_world(*uv)))
            .collect();
        let stroke = egui::Stroke::new(2.0_f32, color);
        for pair in screen.windows(2) {
            painter.line_segment([pair[0], pair[1]], stroke);
        }
        if close && screen.len() >= 3 {
            painter.line_segment([screen[screen.len() - 1], screen[0]], stroke);
        }
        for point in screen {
            painter.circle_filled(point, 4.0, color);
        }
    }

    fn status_text(&self) -> String {
        match self.doc.selection().and_then(|id| self.doc.object(id)) {
            Some(object) => {
                let (w, d, h) = object.size();
                format!(
                    "objects={} n={} selected={} x={:.2} y={:.2} z={:.2} rx={:.1} ry={:.1} rz={:.1} w={:.2} d={:.2} h={:.2} hole={} color={}",
                    self.doc.objects().len(),
                    self.doc.selected_ids().len(),
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
                    if object.hole { "yes" } else { "no" },
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
        let before = self.doc.objects().len();
        let result = self.import_bytes(&name, &bytes);
        match result {
            Ok(()) => {
                self.history.end();
                let added = self.doc.objects().len().saturating_sub(before);
                self.notice = format!("Imported {name} ({added})");
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err;
            }
        }
    }

    fn import_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".3mf") {
            import_3mf(&mut self.doc, bytes).map_err(|err| err.to_string())
        } else if lower.ends_with(".obj") {
            document_from_obj(&mut self.doc, bytes).map_err(|err| err.to_string())
        } else if lower.ends_with(".svg") {
            import_svg(&mut self.doc, bytes).map_err(|err| err.to_string())
        } else if lower.ends_with(".dxf") {
            import_dxf(&mut self.doc, bytes).map_err(|err| err.to_string())
        } else if lower.ends_with(".step") || lower.ends_with(".stp") {
            document_from_step(&mut self.doc, bytes).map_err(|err| err.to_string())
        } else {
            import_stl(&mut self.doc, name, bytes)
                .map(|_| ())
                .map_err(|_| format!("Could not read {name} as STL"))
        }
    }

    fn combine(&mut self, op: BooleanOp) {
        self.history.end();
        self.history.begin(&self.doc);
        match self.doc.boolean_selected(op) {
            Ok(()) => {
                self.history.end();
                self.notice = match op {
                    BooleanOp::Union => "Joined the selection.".into(),
                    BooleanOp::Subtract => "Cut the other bodies out of the primary.".into(),
                    BooleanOp::Intersect => "Kept only the shared volume.".into(),
                };
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err.to_string();
            }
        }
    }

    fn make_group(&mut self) {
        self.history.end();
        self.history.begin(&self.doc);
        match self.doc.group_selected() {
            Ok(_) => {
                self.history.end();
                self.notice = "Grouped. A hole in the group cuts the solids until ungroup.".into();
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err.to_string();
            }
        }
    }

    fn make_ungroup(&mut self) {
        self.history.end();
        self.history.begin(&self.doc);
        match self.doc.ungroup_selected() {
            Ok(()) => {
                self.history.end();
                self.notice = "Ungrouped. The bodies are separate again.".into();
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err.to_string();
            }
        }
    }

    fn toggle_hole(&mut self) {
        let ids = self.doc.selected_ids().to_vec();
        if ids.is_empty() {
            self.notice = "Select a body to mark as a hole.".into();
            return;
        }
        let make_hole = ids
            .iter()
            .any(|id| self.doc.object(*id).is_some_and(|object| !object.hole));
        self.history.end();
        self.history.begin(&self.doc);
        for id in ids {
            self.doc.set_hole(id, make_hole);
        }
        self.history.end();
        self.notice = if make_hole {
            "Marked as a hole. Group it with a solid to cut.".into()
        } else {
            "Marked as a solid.".into()
        };
    }

    fn apply_fillet(&mut self) {
        let Some(id) = self.doc.selection() else {
            self.notice = "Select a box to fillet.".into();
            return;
        };
        self.history.end();
        self.history.begin(&self.doc);
        match self.doc.fillet(id, self.fillet_radius) {
            Ok(()) => {
                self.history.end();
                self.notice = format!("Filleted the box at {:.2} mm.", self.fillet_radius);
            }
            Err(err) => {
                self.history.cancel();
                self.notice = err.to_string();
            }
        }
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

    fn pointer_move(&mut self, rect: Rect, pos: Pos2, shift: bool) {
        match self.gesture {
            Gesture::Move {
                id,
                plane_y,
                grab_x,
                grab_z,
                start_x,
                start_z,
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
                    let x = snap_mm(hit.x - grab_x, step);
                    let z = snap_mm(hit.z - grab_z, step);
                    let dx = x - start_x;
                    let dz = z - start_z;
                    if let Gesture::Move { group, .. } = &self.gesture {
                        let group = group.clone();
                        for (member, sx, sz) in group {
                            self.doc.move_object_to(member, sx + dx, sz + dz);
                        }
                    }
                    let _ = id;
                }
            }
            Gesture::Axis {
                gizmo,
                origin,
                pivot,
                axis_tip,
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
                if let Gesture::Axis { engaged, .. } = &mut self.gesture {
                    *engaged = true;
                }
                let travel = overlay::axis_travel(origin, pos, pivot, axis_tip);
                let step = if self.snap { DEFAULT_SNAP_MM } else { 0.0 };
                let delta = overlay::snapped_axis_delta(travel, step);
                let axis = gizmo.axis() * delta;
                if let Gesture::Axis { starts, .. } = &self.gesture {
                    let starts = starts.clone();
                    for (id, start) in starts {
                        if let Some(object) = self.doc.object_mut(id) {
                            object.position = start + axis;
                        }
                    }
                }
            }
            Gesture::Turn {
                gizmo,
                pivot_screen,
                pivot_world,
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
                if let Gesture::Turn { engaged, .. } = &mut self.gesture {
                    *engaged = true;
                }
                let facing = (self.camera.eye() - pivot_world).dot(gizmo.axis());
                let raw = overlay::screen_arc_degrees(pivot_screen, origin, pos, facing);
                let degrees = overlay::snap_degrees(raw, if shift { 15.0 } else { 1.0 });
                self.apply_turn(gizmo.axis(), pivot_world, degrees);
                self.notice = format!("{degrees:.0}°");
            }
            Gesture::Cube { origin, engaged } => {
                if pos.distance(origin) > DRAG_SLOP || engaged {
                    self.camera.orbit(pos.x - origin.x, pos.y - origin.y);
                    if let Gesture::Cube {
                        origin, engaged, ..
                    } = &mut self.gesture
                    {
                        *origin = pos;
                        *engaged = true;
                    }
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

    fn begin_primary(&mut self, rect: Rect, pos: Pos2, add: bool) {
        if self.sketching && !overlay::view_cube_rect(rect).contains(pos) {
            self.pending_point = self.sketch_uv(rect, pos);
            self.gesture = Gesture::Orbit {
                last: pos,
                origin: pos,
                engaged: false,
            };
            return;
        }
        if overlay::view_cube_rect(rect).contains(pos) {
            self.gesture = Gesture::Cube {
                origin: pos,
                engaged: false,
            };
            return;
        }
        let aspect = Self::aspect_of(rect);
        if let Some((min, max)) = self.doc.selection_bounds() {
            let reach = if self.mobile { 30.0 } else { 18.0 };
            if let Some(gizmo) =
                overlay::hit_gizmo(&self.camera, aspect, rect, min, max, pos, reach)
            {
                self.start_gizmo(rect, pos, gizmo, min, max);
                return;
            }
        }
        let ray = self.ray_at(rect, pos);
        if let Some(id) = self.doc.hit_test(ray) {
            if add {
                self.doc.toggle_select(id);
            } else if !self.doc.is_selected(id) {
                self.doc.select(Some(id));
            }
            let object = self.doc.object(id).unwrap();
            let plane_y = object.position.y;
            let (grab_x, grab_z) = if let Some(hit) = ray_plane_y(ray, plane_y) {
                (hit.x - object.position.x, hit.z - object.position.z)
            } else {
                (0.0, 0.0)
            };
            if !self.doc.is_selected(id) {
                self.gesture = Gesture::None;
                return;
            }
            let group = self
                .doc
                .motion_ids()
                .iter()
                .filter_map(|member| {
                    self.doc
                        .object(*member)
                        .map(|body| (*member, body.position.x, body.position.z))
                })
                .collect();
            self.gesture = Gesture::Move {
                id,
                plane_y,
                grab_x,
                grab_z,
                start_x: object.position.x,
                start_z: object.position.z,
                group,
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
        if self.sketching {
            if let Gesture::Orbit { engaged: false, .. } = self.gesture {
                if let Some(uv) = self.pending_point.take() {
                    if self.draw_path {
                        self.sketch_path.push(uv);
                    } else {
                        self.profile.push(uv);
                    }
                    self.notice = format!(
                        "Profile {} path {} at {:.0}, {:.0}",
                        self.profile.len(),
                        self.sketch_path.len(),
                        uv[0],
                        uv[1]
                    );
                }
            } else {
                self.pending_point = None;
            }
            self.gesture = Gesture::None;
            self.history.end();
            return;
        }
        let cube_click = matches!(self.gesture, Gesture::Cube { engaged: false, .. });
        let cube_at = match self.gesture {
            Gesture::Cube { origin, .. } => Some(origin),
            _ => None,
        };
        match self.gesture {
            Gesture::Move {
                engaged: false, id, ..
            } => {
                if !self.add_select {
                    self.doc.select(Some(id));
                }
            }
            Gesture::Orbit { engaged: false, .. } => self.doc.select(None),
            Gesture::Turn { engaged: true, .. } => {
                self.doc.bake_rotations();
                self.notice = "Rotation baked to 0, 0, 0".into();
            }
            Gesture::Cube { .. } => {}
            _ => {}
        }
        if cube_click {
            if let Some(origin) = cube_at {
                if let Some(face) = overlay::hit_cube_face(&self.cube_faces, origin) {
                    let (yaw, pitch) = face.yaw_pitch();
                    self.camera.set_facing(yaw, pitch);
                    self.notice = "View cube".into();
                }
            }
        }
        self.gesture = Gesture::None;
        self.history.end();
    }

    fn rewind_gesture(&mut self) {
        let places: Vec<(ObjectId, glam::Vec3, Option<glam::Vec3>)> = match &self.gesture {
            Gesture::Move {
                id,
                start_x,
                start_z,
                engaged: true,
                ..
            } => vec![(*id, glam::Vec3::new(*start_x, 0.0, *start_z), None)],
            Gesture::Axis {
                starts,
                engaged: true,
                ..
            } => starts.iter().map(|(id, pos)| (*id, *pos, None)).collect(),
            Gesture::Turn {
                starts,
                engaged: true,
                ..
            } => starts
                .iter()
                .map(|(id, pos, rot)| (*id, *pos, Some(*rot)))
                .collect(),
            _ => return,
        };
        for (id, position, rotation) in places {
            if let Some(object) = self.doc.object_mut(id) {
                if rotation.is_some() {
                    object.position = position;
                    object.rotation = rotation.unwrap();
                } else if matches!(self.gesture, Gesture::Move { .. }) {
                    object.position.x = position.x;
                    object.position.z = position.z;
                } else {
                    object.position = position;
                }
            }
        }
        self.history.cancel();
    }

    fn start_gizmo(
        &mut self,
        rect: Rect,
        pos: Pos2,
        gizmo: Gizmo,
        min: glam::Vec3,
        max: glam::Vec3,
    ) {
        let aspect = Self::aspect_of(rect);
        let center = (min + max) * 0.5;
        let pivot = overlay::project(&self.camera, aspect, rect, center).unwrap_or(pos);
        let axis_tip = overlay::project(&self.camera, aspect, rect, center + gizmo.axis())
            .unwrap_or(pivot + egui::vec2(20.0, 0.0));
        if gizmo.is_rotation() {
            let starts = self
                .doc
                .selected_ids()
                .iter()
                .filter_map(|id| {
                    let object = self.doc.object(*id)?;
                    Some((*id, object.position, object.rotation))
                })
                .collect();
            self.gesture = Gesture::Turn {
                gizmo,
                pivot_screen: overlay::project(&self.camera, aspect, rect, center).unwrap_or(pos),
                pivot_world: center,
                origin: pos,
                starts,
                engaged: false,
            };
        } else {
            let starts = self
                .doc
                .motion_ids()
                .iter()
                .filter_map(|id| self.doc.object(*id).map(|object| (*id, object.position)))
                .collect();
            self.gesture = Gesture::Axis {
                gizmo,
                origin: pos,
                pivot,
                axis_tip,
                starts,
                engaged: false,
            };
        }
    }

    fn apply_turn(&mut self, axis: glam::Vec3, pivot: glam::Vec3, degrees: f32) {
        let Gesture::Turn { starts, .. } = &self.gesture else {
            return;
        };
        let starts = starts.clone();
        let turn = glam::Quat::from_axis_angle(axis, degrees.to_radians());
        for (id, start_pos, start_rot) in starts {
            let Some(object) = self.doc.object(id) else {
                continue;
            };
            let (_, _, height) = object.size();
            let start_center = start_pos + glam::Vec3::new(0.0, height * 0.5, 0.0);
            let new_center = pivot + turn * (start_center - pivot);
            let start_q = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                start_rot.x.to_radians(),
                start_rot.y.to_radians(),
                start_rot.z.to_radians(),
            );
            let (rx, ry, rz) = (turn * start_q).to_euler(glam::EulerRot::XYZ);
            if let Some(object) = self.doc.object_mut(id) {
                object.position = new_center - glam::Vec3::new(0.0, height * 0.5, 0.0);
                object.rotation =
                    glam::Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
            }
        }
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
                self.rewind_gesture();
                let pos = *self.touches.values().next().unwrap();
                if self.touches.len() == 1 && matches!(self.gesture, Gesture::Pinch { .. }) {
                    self.gesture = Gesture::Orbit {
                        last: pos,
                        origin: pos,
                        engaged: true,
                    };
                } else if matches!(self.gesture, Gesture::None) {
                    self.begin_primary(rect, pos, self.add_select);
                }
            }
            _ => {
                self.rewind_gesture();
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
            self.pointer_move(rect, pos, false);
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
                let add = ctx.input(|input| input.modifiers.shift) || self.add_select;
                self.begin_primary(rect, pos, add);
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(pos) = pointer.interact_pos() {
                let shift = ctx.input(|input| input.modifiers.shift);
                self.pointer_move(rect, pos, shift);
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            || (matches!(self.gesture, Gesture::Move { .. } | Gesture::Orbit { .. })
                && pointer.primary_released())
        {
            self.end_primary();
        } else if response.clicked() {
            if let Some(pos) = pointer.interact_pos() {
                let add = ctx.input(|input| input.modifiers.shift) || self.add_select;
                self.begin_primary(rect, pos, add);
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
        self.take_project();
        self.take_drive();
        theme::apply(ctx, self.theme, self.mobile);
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
            self.doc.bake_rotations();
            self.notice = "Undid the last change".into();
        } else if redo_key {
            self.history.redo(&mut self.doc);
            self.doc.bake_rotations();
            self.notice = "Redid the change".into();
        } else if delete_key {
            if self.doc.selection().is_some() {
                self.history.end();
                self.history.begin(&self.doc);
                self.doc.delete_selected();
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
                let scroll_height = (ui.available_height() - 36.0).max(120.0);
                egui::ScrollArea::vertical().max_height(scroll_height).show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading("PrintCaddy");
                    ui.horizontal(|ui| {
                        if ui.button("Projects").clicked() {
                            self.show_projects = true;
                        }
                        if ui.button("Settings").clicked() {
                            self.show_settings = true;
                        }
                    });
                    ui.label(format!("Project {}", self.project_name));
                    ui.label("Colour");
                    ui.horizontal_wrapped(|ui| {
                        for hex in theme::swatches() {
                            let color = printcaddy_core::Color::from_hex(hex);
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
                            ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(color.r, color.g, color.b));
                            if response.clicked() {
                                self.paint_selection(color);
                            }
                        }
                    });
                    ui.small(format!("New bodies stay {}", printcaddy_core::DEFAULT_MODEL_COLOR.hex_string()));
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
                        if ui.button("Import file").clicked() {
                            self.show_import = true;
                        }
                        if ui.button("Export file").clicked() {
                            self.show_export = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        duplicate = ui.button("Duplicate").clicked();
                        delete = ui.button("Delete").clicked();
                    });
                    ui.checkbox(&mut self.snap, "Snap to 1 mm grid");
                    ui.checkbox(&mut self.add_select, "Add to selection");
                    ui.separator();
                    ui.label("Sketch");
                    if self.sketching {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!self.draw_path, "Profile").clicked() {
                                self.draw_path = false;
                            }
                            if ui.selectable_label(self.draw_path, "Path").clicked() {
                                self.draw_path = true;
                            }
                        });
                        number(ui, "Height", &mut self.sketch_height);
                        number(ui, "Angle", &mut self.sketch_angle);
                        ui.horizontal(|ui| {
                            if ui.button("Extrude").clicked() {
                                self.finish_sketch(SketchOp::Extrude);
                            }
                            if ui.button("Revolve").clicked() {
                                self.finish_sketch(SketchOp::Revolve);
                            }
                            if ui.button("Sweep").clicked() {
                                self.finish_sketch(SketchOp::Sweep);
                            }
                        });
                        if ui.button("Cancel").clicked() {
                            self.cancel_sketch();
                        }
                        ui.small(format!(
                            "Profile {} points, path {} points. Click the plate to add one. Drag to orbit.",
                            self.profile.len(),
                            self.sketch_path.len()
                        ));
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Draw sketch").clicked() {
                                self.begin_sketch();
                            }
                            if ui.button("Edit sketch").clicked() {
                                self.edit_selected_sketch();
                            }
                        });
                    }
                    ui.separator();
                    ui.label("Combine");
                    ui.horizontal(|ui| {
                        if ui.button("Join").clicked() {
                            self.combine(BooleanOp::Union);
                        }
                        if ui.button("Subtract").clicked() {
                            self.combine(BooleanOp::Subtract);
                        }
                        if ui.button("Intersect").clicked() {
                            self.combine(BooleanOp::Intersect);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Group").clicked() {
                            self.make_group();
                        }
                        if ui.button("Ungroup").clicked() {
                            self.make_ungroup();
                        }
                        if ui.button("Hole").clicked() {
                            self.toggle_hole();
                        }
                    });
                    number(ui, "Fillet", &mut self.fillet_radius);
                    if ui.button("Fillet box").clicked() {
                        self.apply_fillet();
                    }
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
                if ui.button("Check for updates").clicked() {
                    self.show_updates = true;
                    self.start_update_check();
                }
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
            self.doc.bake_rotations();
            self.notice = "Undid the last change".into();
        }
        if redo {
            self.history.redo(&mut self.doc);
            self.doc.bake_rotations();
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
            if self.doc.selection().is_some() {
                self.history.end();
                self.history.begin(&self.doc);
                self.doc.delete_selected();
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
                let colors = theme::palette(self.theme);
                let grid = std::sync::Arc::new(scene::GridLines::colored(
                    colors.grid_minor,
                    colors.grid_major,
                    colors.grid_border,
                ));
                let frame = scene::draw_frame(
                    &self.doc,
                    self.camera.view_proj(aspect),
                    &mut self.meshes,
                    grid,
                    colors.viewport,
                    colors.xray,
                );
                let callback = self.painter.callback(frame);
                ui.painter().add(egui::PaintCallback { rect, callback });
                if let Some((min, max)) = self.doc.selection_bounds() {
                    overlay::paint_gizmos(ui.painter(), &self.camera, aspect, rect, min, max);
                }
                self.paint_sketch(ui.painter(), aspect, rect);
                let cube = overlay::view_cube_rect(rect);
                self.cube_faces = overlay::paint_view_cube(ui.painter(), &self.camera, cube);
            });
        let _ = view_rect;
        if ctx.input(|input| input.pointer.any_released()) {
            self.doc.bake_rotations();
            self.history.end();
        }
        self.settings_window(ctx);
        self.projects_window(ctx);
        self.updates_window(ctx);
        self.exchange_windows(ctx);
        files::publish_status(&format!(
            "{} | {} theme={} mobile={}",
            self.status_text(),
            self.notice,
            self.theme.label(),
            if self.mobile { "yes" } else { "no" }
        ));
    }
}

impl PrintCaddyApp {
    fn exchange_windows(&mut self, ctx: &egui::Context) {
        if self.show_import {
            let mut open = true;
            let mut pick: Option<(&str, &str)> = None;
            egui::Window::new("Import")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Millimetres. 3MF, OBJ, and STEP are Z-up. SVG and DXF become a sketch you can extrude, revolve, or sweep.");
                    ui.horizontal_wrapped(|ui| {
                        for (label, accept) in [
                            ("STL", ".stl"),
                            ("3MF", ".3mf"),
                            ("OBJ", ".obj"),
                            ("SVG", ".svg"),
                            ("DXF", ".dxf"),
                            ("STEP", ".step,.stp"),
                        ] {
                            if ui.button(label).clicked() {
                                pick = Some((label, accept));
                            }
                        }
                    });
                    ui.small("A file over 32 MB is refused. SVG and DXF stop at 2 MB. Exact CAD STEP is refused; triangulated faces are read.");
                });
            if let Some((_, accept)) = pick {
                files::pick_file(self.incoming.clone(), accept);
                self.show_import = false;
            } else {
                self.show_import = open;
            }
        }
        if self.show_export {
            let mut open = true;
            let mut chosen = None;
            egui::Window::new("Export")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Millimetres, Z-up, matching the TypeScript app. STL stays in the document's axes.");
                    if ui.button("3MF with colour").clicked() {
                        chosen = Some("3mf");
                    }
                    if ui.button("OBJ with colour").clicked() {
                        chosen = Some("obj");
                    }
                    if ui.button("STEP triangles").clicked() {
                        chosen = Some("step");
                    }
                    ui.small("STEP here is triangulated POLY_LOOP faces, not analytic B-rep.");
                });
            if let Some(kind) = chosen {
                self.export_exchange(kind);
                self.show_export = false;
            } else {
                self.show_export = open;
            }
        }
    }

    fn export_exchange(&mut self, kind: &str) {
        let selected = self.doc.selection().is_some();
        match kind {
            "3mf" => match export_3mf(&self.doc, selected) {
                Ok(bytes) => {
                    files::download_named("printcaddy.3mf", "model/3mf", &bytes);
                    self.notice = "Exported 3MF.".into();
                }
                Err(err) => self.notice = err.to_string(),
            },
            "obj" => {
                let text = document_to_obj(&self.doc, selected);
                files::download_text("printcaddy.obj", &text);
                self.notice = "Exported OBJ.".into();
            }
            "step" => match document_to_step(&self.doc, selected) {
                Ok(text) => {
                    files::download_text("printcaddy.step", &text);
                    self.notice = "Exported triangulated STEP.".into();
                }
                Err(err) => self.notice = err.to_string(),
            },
            _ => {}
        }
    }

    fn paint_selection(&mut self, color: printcaddy_core::Color) {
        let ids = self.doc.selected_ids().to_vec();
        if ids.is_empty() {
            self.notice = "Select a body to recolour.".into();
            return;
        }
        self.history.end();
        self.history.begin(&self.doc);
        for id in ids {
            self.doc.set_color(id, color);
        }
        self.history.end();
        self.notice = format!("Painted the selection {}", color.hex_string());
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Theme");
                for theme in ThemeId::ALL {
                    if ui
                        .selectable_label(self.theme == theme, theme.label())
                        .clicked()
                    {
                        self.theme = theme;
                        library::store_settings(theme.label(), self.mobile);
                    }
                }
                if ui.checkbox(&mut self.mobile, "Mobile mode").changed() {
                    library::store_settings(self.theme.label(), self.mobile);
                }
                ui.small("Mobile mode enlarges the handles and the buttons.");
            });
        self.show_settings = open;
    }

    fn projects_window(&mut self, ctx: &egui::Context) {
        if !self.show_projects {
            return;
        }
        let mut open = true;
        let mut save = false;
        let mut export = false;
        let mut import = false;
        let mut open_id = None;
        let mut delete_id = None;
        let mut rename_id = None;
        egui::Window::new("Projects")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.project_name);
                ui.horizontal(|ui| {
                    save = ui.button("Save").clicked();
                    export = ui.button("Export file").clicked();
                    import = ui.button("Import file").clicked();
                });
                ui.separator();
                ui.label("In this browser");
                let projects = self.projects.clone();
                for project in projects {
                    ui.horizontal(|ui| {
                        ui.label(project.name.clone());
                        if ui.button("Open").clicked() {
                            open_id = Some(project.id);
                        }
                        if ui.button("Rename").clicked() {
                            rename_id = Some(project.id);
                        }
                        if ui.button("Delete").clicked() {
                            delete_id = Some(project.id);
                        }
                    });
                }
                ui.separator();
                ui.label(&self.drive_text);
                ui.horizontal(|ui| {
                    if ui.button("Save to Drive").clicked() {
                        drive::save(
                            self.project_name.clone(),
                            printcaddy_core::document_to_json(&self.doc),
                            self.drive_note.clone(),
                        );
                        self.drive_text = "Signing in to Google Drive…".into();
                    }
                    if ui.button("Open from Drive").clicked() {
                        drive::open_latest(self.drive_note.clone(), self.drive_file.clone());
                        self.drive_text = "Signing in to Google Drive…".into();
                    }
                });
                ui.small("Uses the TypeScript site's existing Google client. If Google refuses this site, that client must allow https://rrkennedy69.github.io as a JavaScript origin.");
            });
        if save {
            let id = library::save_current(&mut self.projects, &self.project_name, &self.doc);
            self.notice = format!("Saved {} ({id})", self.project_name);
        }
        if let Some(id) = open_id {
            match library::open_saved(&self.projects, id) {
                Ok(doc) => {
                    let name = self
                        .projects
                        .iter()
                        .find(|project| project.id == id)
                        .map(|project| project.name.clone())
                        .unwrap_or_else(|| "Project".into());
                    self.doc = doc;
                    self.history = History::default();
                    self.project_name = name.clone();
                    self.notice = format!("Opened {name}. Undo starts empty.");
                }
                Err(err) => self.notice = err.to_string(),
            }
        }
        if let Some(id) = rename_id {
            if let Some(project) = self.projects.iter_mut().find(|project| project.id == id) {
                project.name = self.project_name.clone();
            }
            library::store(&self.projects);
            self.notice = format!("Renamed to {}", self.project_name);
        }
        if let Some(id) = delete_id {
            self.projects.retain(|project| project.id != id);
            library::store(&self.projects);
            self.notice = "Deleted the saved project.".into();
        }
        if export {
            let name = format!("{}.printcaddy.json", self.project_name.replace(' ', "-"));
            files::download_text(&name, &printcaddy_core::document_to_json(&self.doc));
            self.notice = "Exported the project file.".into();
        }
        if import {
            files::pick_text(self.project_in.clone());
        }
        self.show_projects = open;
    }

    fn updates_window(&mut self, ctx: &egui::Context) {
        if !self.show_updates {
            return;
        }
        let mut open = true;
        let mut reload = false;
        egui::Window::new("Check for updates")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(&self.update_text);
                if ui.button("Check now").clicked() {
                    self.start_update_check();
                }
                if self.update_newer && ui.button("Reload").clicked() {
                    reload = true;
                }
            });
        if reload {
            reload_page();
        }
        self.show_updates = open;
    }

    fn start_update_check(&mut self) {
        self.update_text = "Checking the published page…".into();
        self.update_newer = false;
        let slot = self.update_slot.clone();
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            *slot.borrow_mut() = Some(fetch_update_report().await);
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            *slot.borrow_mut() = Some("CURRENT This check runs in the browser.".into());
        }
    }

    fn take_project(&mut self) {
        let Some((name, text)) = self.project_in.borrow_mut().take() else {
            return;
        };
        match printcaddy_core::document_from_json(&text) {
            Ok(doc) => {
                self.doc = doc;
                self.history = History::default();
                self.project_name = name
                    .trim_end_matches(".printcaddy.json")
                    .trim_end_matches(".json")
                    .to_string();
                self.notice = format!("Imported {name}. Undo starts empty.");
            }
            Err(err) => self.notice = err.to_string(),
        }
    }

    fn take_drive(&mut self) {
        if let Some(text) = self.drive_note.borrow_mut().take() {
            self.drive_text = text;
        }
        if let Some(body) = self.drive_file.borrow_mut().take() {
            match printcaddy_core::document_from_json(&body) {
                Ok(doc) => {
                    self.doc = doc;
                    self.history = History::default();
                    self.notice = "Opened the project from Google Drive. Undo starts empty.".into();
                }
                Err(err) => self.drive_text = err.to_string(),
            }
        }
        if let Some(report) = self.update_slot.borrow_mut().take() {
            self.update_newer = report.starts_with("NEWER");
            self.update_text = report
                .trim_start_matches("NEWER ")
                .trim_start_matches("CURRENT ")
                .to_string();
            self.notice = self.update_text.clone();
        }
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

fn reload_page() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(path) = window.location().pathname() {
                let fresh = js_sys::Date::now();
                let _ = window.location().set_href(&format!("{path}?fresh={fresh}"));
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_update_report() -> String {
    use printcaddy_core::compare_builds;
    use printcaddy_core::UpdateReport;
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return "CURRENT The browser window is missing.".into();
    };
    let current = window
        .document()
        .and_then(|document| document.document_element())
        .map(|element| element.inner_html())
        .unwrap_or_default();
    let url = format!(
        "https://rrkennedy69.github.io/printcaddy-rs/index.html?fresh={}",
        js_sys::Date::now()
    );
    let init = web_sys::RequestInit::new();
    init.set_cache(web_sys::RequestCache::NoStore);
    let request = match web_sys::Request::new_with_str_and_init(&url, &init) {
        Ok(request) => request,
        Err(_) => return "CURRENT Could not ask the published page.".into(),
    };
    let response =
        match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(response) => response,
            Err(_) => return "CURRENT Could not reach the published page.".into(),
        };
    let Ok(response) = response.dyn_into::<web_sys::Response>() else {
        return "CURRENT Could not read the published page.".into();
    };
    if !response.ok() {
        return format!(
            "CURRENT The published page returned HTTP {}.",
            response.status()
        );
    }
    let Ok(text_promise) = response.text() else {
        return "CURRENT Could not read the published page.".into();
    };
    let Ok(text) = wasm_bindgen_futures::JsFuture::from(text_promise).await else {
        return "CURRENT Could not read the published page.".into();
    };
    let remote = text.as_string().unwrap_or_default();
    match compare_builds(&current, &remote) {
        UpdateReport::Current => "CURRENT This is the current build.".into(),
        UpdateReport::Newer { remote, .. } => {
            format!("NEWER A newer build is published ({remote}).")
        }
        UpdateReport::Unknown => "CURRENT The published page did not name a build.".into(),
    }
}

fn ray_onto_plane(ray: Ray, origin: glam::Vec3, normal: glam::Vec3) -> Option<glam::Vec3> {
    let normal = normal.normalize_or_zero();
    if normal.length_squared() < 1e-8 {
        return None;
    }
    let denom = ray.direction.dot(normal);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (origin - ray.origin).dot(normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + ray.direction * t)
}

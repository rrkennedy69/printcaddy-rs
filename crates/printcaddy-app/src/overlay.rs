//! Screen-space handles, dimension lines, and the view cube.
//! Drawn after the mesh, so a body cannot hide a rotation handle.

use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use glam::Vec3;
use printcaddy_core::snap_mm;

use crate::camera::Camera;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gizmo {
    MoveX,
    MoveY,
    MoveZ,
    RotX,
    RotY,
    RotZ,
}

impl Gizmo {
    pub fn axis(self) -> Vec3 {
        match self {
            Gizmo::MoveX | Gizmo::RotX => Vec3::X,
            Gizmo::MoveY | Gizmo::RotY => Vec3::Y,
            Gizmo::MoveZ | Gizmo::RotZ => Vec3::Z,
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Gizmo::MoveX | Gizmo::RotX => Color32::from_rgb(214, 64, 64),
            Gizmo::MoveY | Gizmo::RotY => Color32::from_rgb(46, 170, 78),
            Gizmo::MoveZ | Gizmo::RotZ => Color32::from_rgb(52, 120, 220),
        }
    }

    pub fn is_rotation(self) -> bool {
        matches!(self, Gizmo::RotX | Gizmo::RotY | Gizmo::RotZ)
    }
}

pub struct GizmoMark {
    pub kind: Gizmo,
    pub anchor: Vec3,
    pub from: Vec3,
}

pub fn gizmo_marks(min: Vec3, max: Vec3) -> [GizmoMark; 6] {
    let center = (min + max) * 0.5;
    let lift = 14.0;
    [
        GizmoMark {
            kind: Gizmo::MoveX,
            from: Vec3::new(max.x, center.y, center.z),
            anchor: Vec3::new(max.x + lift, center.y, center.z),
        },
        GizmoMark {
            kind: Gizmo::MoveY,
            from: Vec3::new(center.x, max.y, center.z),
            anchor: Vec3::new(center.x, max.y + lift, center.z),
        },
        GizmoMark {
            kind: Gizmo::MoveZ,
            from: Vec3::new(center.x, center.y, max.z),
            anchor: Vec3::new(center.x, center.y, max.z + lift),
        },
        // Red and green rotation grips sit on the top. Blue sits under the body.
        GizmoMark {
            kind: Gizmo::RotX,
            from: Vec3::new(max.x, max.y, center.z),
            anchor: Vec3::new(max.x, max.y + lift, center.z),
        },
        GizmoMark {
            kind: Gizmo::RotY,
            from: Vec3::new(center.x, max.y, max.z),
            anchor: Vec3::new(center.x, max.y + lift, max.z),
        },
        GizmoMark {
            kind: Gizmo::RotZ,
            from: Vec3::new(center.x, min.y, min.z),
            anchor: Vec3::new(center.x, min.y - lift, min.z),
        },
    ]
}

pub fn project(camera: &Camera, aspect: f32, rect: Rect, world: Vec3) -> Option<Pos2> {
    let clip = camera.view_proj(aspect) * world.extend(1.0);
    if clip.w <= 0.05 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(Pos2::new(
        rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width(),
        rect.min.y + (0.5 - ndc.y * 0.5) * rect.height(),
    ))
}

pub fn axis_travel(origin: Pos2, pos: Pos2, pivot: Pos2, axis_tip: Pos2) -> f32 {
    let screen = axis_tip - pivot;
    let len = screen.length();
    if len < 2.0 {
        return 0.0;
    }
    (pos - origin).dot(screen / len) / len
}

pub fn snapped_axis_delta(travel: f32, step: f32) -> f32 {
    snap_mm(travel, step)
}

pub fn screen_arc_degrees(pivot: Pos2, start: Pos2, current: Pos2, facing: f32) -> f32 {
    let a = (start - pivot).y.atan2((start - pivot).x);
    let b = (current - pivot).y.atan2((current - pivot).x);
    let mut delta = b - a;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    let sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    delta.to_degrees() * sign
}

pub fn snap_degrees(degrees: f32, step: f32) -> f32 {
    if step <= 0.0 {
        degrees
    } else {
        (degrees / step).round() * step
    }
}

pub fn paint_gizmos(
    painter: &egui::Painter,
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    min: Vec3,
    max: Vec3,
) {
    let center = (min + max) * 0.5;
    for mark in gizmo_marks(min, max) {
        let Some(from) = project(camera, aspect, rect, mark.from) else {
            continue;
        };
        let Some(at) = project(camera, aspect, rect, mark.anchor) else {
            continue;
        };
        let color = mark.kind.color();
        if mark.kind.is_rotation() {
            paint_arc(painter, at, color);
        } else {
            painter.line_segment([from, at], Stroke::new(3.0_f32, color));
            paint_head(painter, from, at, color);
        }
    }
    paint_dimensions(painter, camera, aspect, rect, min, max, center);
}

fn paint_dimensions(
    painter: &egui::Painter,
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    min: Vec3,
    max: Vec3,
    _center: Vec3,
) {
    let width = max.x - min.x;
    let depth = max.z - min.z;
    let height = max.y - min.y;
    let y = min.y;
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(min.x, y, max.z + 8.0),
        Vec3::new(max.x, y, max.z + 8.0),
        format!("{width:.1}"),
    );
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(max.x + 8.0, y, min.z),
        Vec3::new(max.x + 8.0, y, max.z),
        format!("{depth:.1}"),
    );
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(max.x + 8.0, min.y, max.z),
        Vec3::new(max.x + 8.0, max.y, max.z),
        format!("{height:.1}"),
    );
}

fn dimension(
    painter: &egui::Painter,
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    a: Vec3,
    b: Vec3,
    label: String,
) {
    let (Some(pa), Some(pb)) = (
        project(camera, aspect, rect, a),
        project(camera, aspect, rect, b),
    ) else {
        return;
    };
    let ink = Color32::from_rgb(230, 220, 180);
    painter.line_segment([pa, pb], Stroke::new(1.5_f32, ink));
    painter.text(
        pa.lerp(pb, 0.5) + Vec2::new(0.0, -10.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        ink,
    );
}

fn paint_arc(painter: &egui::Painter, at: Pos2, color: Color32) {
    let radius = 16.0;
    let mut points = Vec::new();
    for step in 0..=10 {
        let angle = -0.9 + 1.8 * (step as f32) / 10.0;
        points.push(at + Vec2::new(angle.cos(), angle.sin()) * radius);
    }
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(3.5_f32, color),
    ));
    if let (Some(prev), Some(end)) = (points.get(points.len() - 2), points.last()) {
        paint_head(painter, *prev, *end, color);
    }
    painter.circle_filled(at, 5.0, color);
    painter.circle_stroke(at, 7.0, Stroke::new(2.0_f32, Color32::WHITE));
}

fn paint_head(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    let dir = to - from;
    let len = dir.length().max(0.001);
    let u = dir / len;
    let n = Vec2::new(-u.y, u.x);
    let tip = to;
    let base = to - u * 10.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, base + n * 5.0, base - n * 5.0],
        color,
        Stroke::NONE,
    ));
}

pub fn hit_gizmo(
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    min: Vec3,
    max: Vec3,
    pointer: Pos2,
) -> Option<Gizmo> {
    let mut best: Option<(f32, Gizmo)> = None;
    for mark in gizmo_marks(min, max) {
        let Some(at) = project(camera, aspect, rect, mark.anchor) else {
            continue;
        };
        let distance = at.distance(pointer);
        if distance < 18.0 && best.map(|(so_far, _)| distance < so_far).unwrap_or(true) {
            best = Some((distance, mark.kind));
        }
    }
    best.map(|(_, kind)| kind)
}

pub fn view_cube_rect(view: Rect) -> Rect {
    let size = 112.0;
    Rect::from_min_size(
        Pos2::new(view.max.x - size - 16.0, view.min.y + 16.0),
        Vec2::splat(size),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeFace {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

impl CubeFace {
    pub fn yaw_pitch(self) -> (f32, f32) {
        let mild = 0.35;
        match self {
            CubeFace::Front => (0.0, mild),
            CubeFace::Back => (std::f32::consts::PI, mild),
            CubeFace::Right => (std::f32::consts::FRAC_PI_2, mild),
            CubeFace::Left => (-std::f32::consts::FRAC_PI_2, mild),
            CubeFace::Top => (0.85, 1.35),
            CubeFace::Bottom => (0.85, -1.15),
        }
    }
}

pub fn paint_view_cube(
    painter: &egui::Painter,
    camera: &Camera,
    area: Rect,
) -> Vec<(CubeFace, Pos2)> {
    let center = area.center();
    let eye = (camera.eye() - camera.target).normalize_or_zero();
    let view = glam::Mat4::look_at_rh(eye * 4.0, Vec3::ZERO, Vec3::Y);
    let half = 0.72;
    let corners = [
        Vec3::new(-half, -half, -half),
        Vec3::new(half, -half, -half),
        Vec3::new(half, half, -half),
        Vec3::new(-half, half, -half),
        Vec3::new(-half, -half, half),
        Vec3::new(half, -half, half),
        Vec3::new(half, half, half),
        Vec3::new(-half, half, half),
    ];
    let projected: Vec<Pos2> = corners
        .iter()
        .map(|corner| {
            let clip = view * corner.extend(1.0);
            let ndc = clip.truncate() / clip.w.max(0.1);
            Pos2::new(center.x + ndc.x * 34.0, center.y - ndc.y * 34.0)
        })
        .collect();
    let faces = [
        (
            CubeFace::Front,
            [4, 5, 6, 7],
            Color32::from_rgb(186, 198, 210),
        ),
        (
            CubeFace::Back,
            [1, 0, 3, 2],
            Color32::from_rgb(150, 162, 174),
        ),
        (
            CubeFace::Right,
            [5, 1, 2, 6],
            Color32::from_rgb(214, 120, 120),
        ),
        (
            CubeFace::Left,
            [0, 4, 7, 3],
            Color32::from_rgb(120, 150, 210),
        ),
        (
            CubeFace::Top,
            [3, 7, 6, 2],
            Color32::from_rgb(230, 234, 236),
        ),
        (
            CubeFace::Bottom,
            [0, 1, 5, 4],
            Color32::from_rgb(130, 140, 148),
        ),
    ];
    let mut order: Vec<(f32, CubeFace, [usize; 4], Color32)> = faces
        .into_iter()
        .map(|(face, idx, color)| {
            let depth = idx
                .iter()
                .map(|i| (view * corners[*i].extend(1.0)).z)
                .sum::<f32>();
            (depth, face, idx, color)
        })
        .collect();
    order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut centers = Vec::new();
    for (_, face, idx, color) in order {
        let poly: Vec<Pos2> = idx.iter().map(|i| projected[*i]).collect();
        painter.add(egui::Shape::convex_polygon(
            poly.clone(),
            color,
            Stroke::new(1.2_f32, Color32::from_rgb(40, 48, 56)),
        ));
        let mid = poly.iter().fold(Pos2::ZERO, |acc, p| acc + p.to_vec2()) / poly.len() as f32;
        centers.push((face, mid));
        let label = match face {
            CubeFace::Front => "F",
            CubeFace::Back => "B",
            CubeFace::Left => "L",
            CubeFace::Right => "R",
            CubeFace::Top => "T",
            CubeFace::Bottom => "U",
        };
        painter.text(
            mid,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(30, 36, 42),
        );
    }
    centers
}

pub fn hit_cube_face(centers: &[(CubeFace, Pos2)], pointer: Pos2) -> Option<CubeFace> {
    centers
        .iter()
        .filter(|(_, at)| at.distance(pointer) < 22.0)
        .min_by(|a, b| {
            a.1.distance(pointer)
                .partial_cmp(&b.1.distance(pointer))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(face, _)| *face)
}

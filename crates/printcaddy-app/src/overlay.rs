//! Screen-space handles, dimension lines, and the view cube.
//! Drawn after the mesh, so a body cannot hide a rotation handle.

use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use glam::Vec3;
use printcaddy_core::{snap_mm, PLATE_SIZE_MM};

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
    unit: f32,
    suffix: &str,
) {
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
    paint_dimensions(painter, camera, aspect, rect, min, max, unit, suffix);
    for index in 0..8 {
        let Some(at) = project(camera, aspect, rect, corner_world(min, max, index)) else {
            continue;
        };
        painter.rect_filled(
            Rect::from_center_size(at, Vec2::splat(9.0)),
            1.5,
            Color32::WHITE,
        );
        painter.rect_stroke(
            Rect::from_center_size(at, Vec2::splat(9.0)),
            1.5,
            Stroke::new(1.4_f32, Color32::from_rgb(32, 40, 52)),
            egui::StrokeKind::Inside,
        );
    }
}

pub fn corner_world(min: Vec3, max: Vec3, index: usize) -> Vec3 {
    Vec3::new(
        if index & 1 == 0 { min.x } else { max.x },
        if index & 2 == 0 { min.y } else { max.y },
        if index & 4 == 0 { min.z } else { max.z },
    )
}

pub fn hit_corner(
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    min: Vec3,
    max: Vec3,
    pointer: Pos2,
    hit_radius: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for index in 0..8 {
        let Some(at) = project(camera, aspect, rect, corner_world(min, max, index)) else {
            continue;
        };
        let distance = at.distance(pointer);
        if distance < hit_radius && best.map(|(so_far, _)| distance < so_far).unwrap_or(true) {
            best = Some((distance, index));
        }
    }
    best.map(|(_, index)| index)
}

fn paint_dimensions(
    painter: &egui::Painter,
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    min: Vec3,
    max: Vec3,
    unit: f32,
    suffix: &str,
) {
    let unit = if unit.abs() < 1.0e-6 { 1.0 } else { unit };
    let width = (max.x - min.x) / unit;
    let depth = (max.z - min.z) / unit;
    let height = (max.y - min.y) / unit;
    let suffix = suffix;
    let y = min.y;
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(min.x, y, max.z + 8.0),
        Vec3::new(max.x, y, max.z + 8.0),
        format!("{width:.1} {suffix}"),
    );
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(max.x + 8.0, y, min.z),
        Vec3::new(max.x + 8.0, y, max.z),
        format!("{depth:.1} {suffix}"),
    );
    dimension(
        painter,
        camera,
        aspect,
        rect,
        Vec3::new(max.x + 8.0, min.y, max.z),
        Vec3::new(max.x + 8.0, max.y, max.z),
        format!("{height:.1} {suffix}"),
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
    let ink = Color32::from_rgb(28, 40, 58);
    painter.line_segment([pa, pb], Stroke::new(1.5_f32, ink));
    let font = egui::FontId::proportional(13.0);
    let galley = painter.layout_no_wrap(label, font, ink);
    let text_rect = Rect::from_center_size(pa.lerp(pb, 0.5) + Vec2::new(0.0, -12.0), galley.size());
    painter.rect_filled(
        text_rect.expand(3.0),
        3.0,
        Color32::from_rgba_premultiplied(255, 250, 240, 230),
    );
    painter.galley(text_rect.min, galley, ink);
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
    hit_radius: f32,
) -> Option<Gizmo> {
    let mut best: Option<(f32, Gizmo)> = None;
    for mark in gizmo_marks(min, max) {
        let Some(at) = project(camera, aspect, rect, mark.anchor) else {
            continue;
        };
        let distance = at.distance(pointer);
        if distance < hit_radius && best.map(|(so_far, _)| distance < so_far).unwrap_or(true) {
            best = Some((distance, mark.kind));
        }
    }
    best.map(|(_, kind)| kind)
}

pub fn view_cube_rect(view: Rect) -> Rect {
    let size = 118.0;
    Rect::from_min_size(
        Pos2::new(view.min.x + 10.0, view.min.y + 8.0),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeTarget {
    Face(CubeFace),
    Edge(CubeFace, CubeFace),
    Corner(CubeFace, CubeFace, CubeFace),
}

impl CubeTarget {
    pub fn yaw_pitch(self) -> (f32, f32) {
        match self {
            CubeTarget::Face(face) => face.yaw_pitch(),
            CubeTarget::Edge(a, b) => blend_views(&[a, b]),
            CubeTarget::Corner(a, b, c) => blend_views(&[a, b, c]),
        }
    }
}

fn blend_views(faces: &[CubeFace]) -> (f32, f32) {
    let mut sx = 0.0;
    let mut sz = 0.0;
    let mut pitch = 0.0;
    let mut planar = 0.0;
    for face in faces {
        match face {
            CubeFace::Top => pitch += 0.95,
            CubeFace::Bottom => pitch -= 0.75,
            other => {
                let (yaw, _) = other.yaw_pitch();
                sx += yaw.sin();
                sz += yaw.cos();
                planar += 1.0;
            }
        }
    }
    let yaw = if planar > 0.0 { sx.atan2(sz) } else { 0.6 };
    let pitch: f32 = if planar > 0.0 { pitch + 0.28 } else { pitch };
    (yaw, pitch.clamp(-1.2, 1.25))
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
) -> Vec<(CubeTarget, Pos2)> {
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
            Color32::from_rgb(255, 252, 247),
        ),
        (
            CubeFace::Back,
            [1, 0, 3, 2],
            Color32::from_rgb(236, 230, 220),
        ),
        (
            CubeFace::Right,
            [5, 1, 2, 6],
            Color32::from_rgb(250, 246, 238),
        ),
        (
            CubeFace::Left,
            [0, 4, 7, 3],
            Color32::from_rgb(244, 238, 228),
        ),
        (
            CubeFace::Top,
            [3, 7, 6, 2],
            Color32::from_rgb(255, 253, 249),
        ),
        (
            CubeFace::Bottom,
            [0, 1, 5, 4],
            Color32::from_rgb(228, 220, 208),
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
    let nearest: Vec<CubeFace> = order.iter().rev().take(3).map(|item| item.1).collect();
    let mut centers = Vec::new();
    for (_, face, idx, color) in order {
        let poly: Vec<Pos2> = idx.iter().map(|i| projected[*i]).collect();
        painter.add(egui::Shape::convex_polygon(
            poly.clone(),
            color,
            Stroke::new(1.0_f32, Color32::from_rgb(176, 156, 128)),
        ));
        let mid = poly.iter().fold(Pos2::ZERO, |acc, p| acc + p.to_vec2()) / poly.len() as f32;
        centers.push((CubeTarget::Face(face), mid));
        if nearest.contains(&face) {
            let label = match face {
                CubeFace::Front => "FRONT",
                CubeFace::Back => "BACK",
                CubeFace::Left => "LEFT",
                CubeFace::Right => "RIGHT",
                CubeFace::Top => "TOP",
                CubeFace::Bottom => "BOTTOM",
            };
            painter.text(
                mid,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(9.0),
                Color32::from_rgb(120, 100, 78),
            );
        }
    }
    for index in 0..8 {
        let faces = mesh_corner_faces(index);
        centers.push((
            CubeTarget::Corner(faces[0], faces[1], faces[2]),
            projected[index],
        ));
        painter.circle_filled(projected[index], 3.5, Color32::from_rgb(40, 48, 56));
    }
    let edges = [
        (0, 1),
        (0, 3),
        (0, 4),
        (1, 2),
        (1, 5),
        (2, 3),
        (2, 6),
        (3, 7),
        (4, 5),
        (4, 7),
        (5, 6),
        (6, 7),
    ];
    for (a, b) in edges {
        let fa = mesh_corner_faces(a);
        let fb = mesh_corner_faces(b);
        let shared: Vec<CubeFace> = fa.into_iter().filter(|face| fb.contains(face)).collect();
        if shared.len() == 2 {
            let mid = Pos2::new(
                (projected[a].x + projected[b].x) * 0.5,
                (projected[a].y + projected[b].y) * 0.5,
            );
            centers.push((CubeTarget::Edge(shared[0], shared[1]), mid));
        }
    }
    centers
}

fn mesh_corner_faces(index: usize) -> [CubeFace; 3] {
    match index {
        0 => [CubeFace::Left, CubeFace::Bottom, CubeFace::Back],
        1 => [CubeFace::Right, CubeFace::Bottom, CubeFace::Back],
        2 => [CubeFace::Right, CubeFace::Top, CubeFace::Back],
        3 => [CubeFace::Left, CubeFace::Top, CubeFace::Back],
        4 => [CubeFace::Left, CubeFace::Bottom, CubeFace::Front],
        5 => [CubeFace::Right, CubeFace::Bottom, CubeFace::Front],
        6 => [CubeFace::Right, CubeFace::Top, CubeFace::Front],
        _ => [CubeFace::Left, CubeFace::Top, CubeFace::Front],
    }
}

pub fn hit_cube(centers: &[(CubeTarget, Pos2)], pointer: Pos2) -> Option<CubeTarget> {
    let mut best: Option<(u8, f32, CubeTarget)> = None;
    for (target, at) in centers {
        let (rank, limit) = match target {
            CubeTarget::Corner(_, _, _) => (0, 14.0),
            CubeTarget::Edge(_, _) => (1, 12.0),
            CubeTarget::Face(_) => (2, 22.0),
        };
        let distance = at.distance(pointer);
        if distance >= limit {
            continue;
        }
        let better = best
            .map(|(so_rank, so_far, _)| rank < so_rank || (rank == so_rank && distance < so_far))
            .unwrap_or(true);
        if better {
            best = Some((rank, distance, *target));
        }
    }
    best.map(|(_, _, target)| target)
}

/// Numbered ticks along the near edges of each plate, plus the design name on the bed.
pub fn paint_rulers(
    painter: &egui::Painter,
    camera: &Camera,
    aspect: f32,
    rect: Rect,
    centers: &[(f32, f32)],
    design: &str,
) {
    let half = PLATE_SIZE_MM * 0.5;
    let red = Color32::from_rgb(176, 64, 52);
    let green = Color32::from_rgb(46, 130, 72);
    for &(ox, oz) in centers {
        for step in 1..=12 {
            let mark = step as f32 * 10.0;
            if mark > half - 2.0 {
                break;
            }
            let x_at = Vec3::new(ox - half + mark, 0.4, oz + half + 2.0);
            if let Some(screen) = project(camera, aspect, rect, x_at) {
                painter.text(
                    screen,
                    egui::Align2::CENTER_TOP,
                    format!("{}", step * 10),
                    egui::FontId::proportional(10.0),
                    red,
                );
            }
            let z_at = Vec3::new(ox - half - 2.0, 0.4, oz - half + mark);
            if let Some(screen) = project(camera, aspect, rect, z_at) {
                painter.text(
                    screen,
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", step * 10),
                    egui::FontId::proportional(10.0),
                    green,
                );
            }
        }
        let center = Vec3::new(ox, 0.3, oz);
        let Some(at) = project(camera, aspect, rect, center) else {
            continue;
        };
        let Some(along) = project(camera, aspect, rect, center + Vec3::X * 40.0) else {
            continue;
        };
        let angle = (along - at).angle();
        let galley = painter.layout_no_wrap(
            design.to_string(),
            egui::FontId::proportional(42.0),
            Color32::from_rgb(120, 128, 134),
        );
        let mut shape = egui::epaint::TextShape::new(at, galley, Color32::from_rgb(120, 128, 134));
        shape.angle = angle;
        shape.opacity_factor = 0.16;
        painter.add(shape);
    }
}

pub fn paint_protractor(painter: &egui::Painter, pivot: Pos2) {
    let radius = 72.0;
    let ink = Color32::from_rgb(36, 44, 58);
    painter.circle_stroke(pivot, radius, Stroke::new(1.4_f32, ink));
    for step in 0..24 {
        let angle = step as f32 / 24.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let inner = if step % 6 == 0 { radius - 14.0 } else { radius - 7.0 };
        painter.line_segment([pivot + dir * inner, pivot + dir * radius], Stroke::new(1.2_f32, ink));
        if step % 6 == 0 {
            painter.text(
                pivot + dir * (radius + 14.0),
                egui::Align2::CENTER_CENTER,
                format!("{}", step * 15),
                egui::FontId::proportional(11.0),
                ink,
            );
        }
    }
}

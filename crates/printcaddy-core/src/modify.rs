//! Mirror, pattern, chamfer, hollow, and split. Each edits the selection in one step.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::boolean::{boolean, mesh_volume, BooleanOp};
use crate::document::{Document, ObjectId, ObjectKind};
use crate::mesh::Mesh;
use crate::sketch::is_closed;
use crate::AlignAxis;

impl Document {
    /// Reflect every selected body across the centre of the selection on `axis`.
    pub fn mirror_selection(&mut self, axis: AlignAxis, copy: bool) -> usize {
        if copy {
            let roots: Vec<_> = self
                .selected_ids()
                .iter()
                .filter_map(|id| self.object(*id).filter(|object| object.parent.is_none() && !object.locked).cloned())
                .collect();
            let copies: Vec<ObjectId> = roots.iter().map(|source| self.place_copy(source, source.position)).collect();
            self.replace_selection(copies);
        }
        let ids: Vec<ObjectId> = self
            .selected_ids()
            .iter()
            .copied()
            .filter(|id| self.object(*id).is_some_and(|object| !object.locked))
            .collect();
        let Some((min, max)) = union_bounds(self, &ids) else {
            return 0;
        };
        let pivot = match axis {
            AlignAxis::X => (min.x + max.x) * 0.5,
            AlignAxis::Y => (min.y + max.y) * 0.5,
            AlignAxis::Z => (min.z + max.z) * 0.5,
        };
        let mut moved = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            let mut mesh = object.world_mesh();
            let slot = axis_index(axis);
            for point in &mut mesh.positions {
                point[slot] = 2.0 * pivot - point[slot];
            }
            for tri in mesh.indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }
            recompute_normals(&mut mesh);
            self.reseat_mesh(id, mesh);
            moved += 1;
        }
        moved
    }

    /// Repeat the selection `count` times, the original included, stepping by `spacing` in scene axes.
    pub fn pattern_row(&mut self, count: u32, spacing: Vec3) -> Vec<ObjectId> {
        let count = count.clamp(2, 100);
        let roots: Vec<_> = self
            .selected_ids()
            .iter()
            .filter_map(|id| self.object(*id).filter(|object| object.parent.is_none() && !object.locked).cloned())
            .collect();
        if roots.is_empty() {
            return Vec::new();
        }
        let mut created = Vec::new();
        for index in 1..count {
            for source in &roots {
                let id = self.place_copy(source, source.position + spacing * index as f32);
                created.push(id);
            }
        }
        let mut all: Vec<ObjectId> = roots.iter().map(|object| object.id).collect();
        all.extend(created.iter().copied());
        self.replace_selection(all);
        created
    }

    /// Bevel every sharp edge of each selected body by `distance` millimetres.
    /// A box gets all twelve edges. A cylinder gets both rims. One step for the selection.
    pub fn chamfer_selection(&mut self, distance: f32) -> usize {
        let distance = distance.max(0.05);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let mesh = match &object.kind {
                ObjectKind::Cylinder { radius, height, segments } => {
                    let chamfer = distance.min(radius * 0.45).min(height * 0.45);
                    if chamfer < 0.05 {
                        continue;
                    }
                    chamfered_cylinder(*radius, *height, *segments, chamfer)
                }
                _ => {
                    let source = object.local_mesh();
                    let Some(mesh) = chamfer_sharp(&source, distance) else {
                        continue;
                    };
                    mesh
                }
            };
            if let Some(object) = self.object_mut(id) {
                object.kind = ObjectKind::Mesh { mesh };
                object.revision = object.revision.saturating_add(1);
                done += 1;
            }
        }
        done
    }

    /// Hollow each selected solid: walls and a floor of `wall` millimetres, open at the top.
    pub fn hollow_selection(&mut self, wall: f32) -> usize {
        let wall = wall.max(0.2);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let outer = object.local_mesh();
            let (min, max) = outer.bounds();
            let width = max.x - min.x;
            let depth = max.z - min.z;
            let height = max.y - min.y;
            if wall * 2.0 >= width.min(depth) || wall >= height || outer.triangle_count() == 0 {
                continue;
            }
            let mut inner = outer.clone();
            for point in &mut inner.positions {
                point[0] = min.x + wall + (point[0] - min.x) * (width - 2.0 * wall) / width;
                point[2] = min.z + wall + (point[2] - min.z) * (depth - 2.0 * wall) / depth;
                point[1] = min.y + wall + (point[1] - min.y) * (height - wall) / height;
            }
            let Ok(mesh) = boolean(BooleanOp::Subtract, &outer, &inner) else {
                continue;
            };
            if let Some(object) = self.object_mut(id) {
                object.kind = ObjectKind::Mesh { mesh };
                object.revision = object.revision.saturating_add(1);
                done += 1;
            }
        }
        done
    }

    /// Cut each selected body on `axis`. `fraction` is 0 at the low side and 1 at the high side.
    pub fn split_selection(&mut self, axis: AlignAxis, fraction: f32) -> usize {
        let ids: Vec<ObjectId> = self
            .selected_ids()
            .iter()
            .copied()
            .filter(|id| self.object(*id).is_some_and(|object| !object.locked))
            .collect();
        let mut made = 0;
        for id in ids {
            let Some(object) = self.object(id).cloned() else {
                continue;
            };
            let mesh = object.local_mesh();
            let (min, max) = mesh.bounds();
            let fraction = fraction.clamp(0.05, 0.95);
            let plane = match axis {
                AlignAxis::X => min.x + (max.x - min.x) * fraction,
                AlignAxis::Y => min.y + (max.y - min.y) * fraction,
                AlignAxis::Z => min.z + (max.z - min.z) * fraction,
            };
            let normal = match axis {
                AlignAxis::X => Vec3::X,
                AlignAxis::Y => Vec3::Y,
                AlignAxis::Z => Vec3::Z,
            };
            let (low, high) = clip_plane(&mesh, normal, normal * plane);
            if low.triangle_count() == 0 || high.triangle_count() == 0 || !is_closed(&low) || !is_closed(&high) {
                continue;
            }
            let base_name = object.name.clone();
            if let Some(object) = self.object_mut(id) {
                object.kind = ObjectKind::Mesh { mesh: low };
                object.revision = object.revision.saturating_add(1);
                object.name = format!("{base_name} A");
            }
            let mut twin = object;
            twin.kind = ObjectKind::Mesh { mesh: high };
            let new_id = self.place_copy(&twin, twin.position);
            if let Some(object) = self.object_mut(new_id) {
                object.name = format!("{base_name} B");
            }
            made += 1;
        }
        made
    }

    /// Cut each selected body with a plane through `origin` whose outward normal points at the high piece.
    pub fn split_on_plane(&mut self, origin: Vec3, normal: Vec3) -> usize {
        let normal = normal.normalize_or_zero();
        if normal.length_squared() < 0.5 {
            return 0;
        }
        let ids: Vec<ObjectId> = self
            .selected_ids()
            .iter()
            .copied()
            .filter(|id| self.object(*id).is_some_and(|object| !object.locked))
            .collect();
        let mut made = 0;
        for id in ids {
            let Some(object) = self.object(id).cloned() else {
                continue;
            };
            let model = object.model_matrix();
            let inv = model.inverse();
            let local_origin = inv.transform_point3(origin);
            let local_normal = inv.transform_vector3(normal).normalize_or_zero();
            if local_normal.length_squared() < 0.5 {
                continue;
            }
            let mesh = object.local_mesh();
            let (low, high) = clip_plane(&mesh, local_normal, local_origin);
            if low.triangle_count() == 0 || high.triangle_count() == 0 || !is_closed(&low) || !is_closed(&high) {
                continue;
            }
            let base_name = object.name.clone();
            if let Some(object) = self.object_mut(id) {
                object.kind = ObjectKind::Mesh { mesh: low };
                object.revision = object.revision.saturating_add(1);
                object.name = format!("{base_name} A");
            }
            let mut twin = object;
            twin.kind = ObjectKind::Mesh { mesh: high };
            let new_id = self.place_copy(&twin, twin.position);
            if let Some(object) = self.object_mut(new_id) {
                object.name = format!("{base_name} B");
            }
            made += 1;
        }
        made
    }

    /// Repeat around a vertical circle. `center` is the scene point the copies turn about. 360° does not land a copy on the original.
    pub fn pattern_circle(&mut self, count: u32, angle: f32, center: Vec3, rotate_copies: bool) -> Vec<ObjectId> {
        let count = count.clamp(2, 100);
        let full = (angle.abs() - 360.0).abs() < 1e-3 || angle.abs() > 360.0;
        let sign = if angle < 0.0 { -1.0 } else { 1.0 };
        let step = if full {
            360.0 / count as f32 * sign
        } else {
            angle / (count as f32 - 1.0)
        };
        let roots: Vec<_> = self
            .selected_ids()
            .iter()
            .filter_map(|id| self.object(*id).filter(|object| object.parent.is_none() && !object.locked).cloned())
            .collect();
        if roots.is_empty() {
            return Vec::new();
        }
        let mut created = Vec::new();
        for index in 1..count {
            let degrees = step * index as f32;
            for source in &roots {
                let (x, z) = turn_xz(source.position.x, source.position.z, center.x, center.z, degrees);
                let id = self.place_copy(source, Vec3::new(x, source.position.y, z));
                if rotate_copies {
                    if let Some(object) = self.object_mut(id) {
                        object.rotation.y += degrees;
                    }
                }
                created.push(id);
            }
        }
        let mut all: Vec<ObjectId> = roots.iter().map(|object| object.id).collect();
        all.extend(created.iter().copied());
        self.replace_selection(all);
        if count > 5 {
            let _ = self.group_selected();
        }
        created
    }

    /// Cut a staggered pattern of round holes out of each selected body. Defaults match the live pattern: 6 mm, 9 mm apart, 4 mm from the edge.
    pub fn hole_pattern(&mut self, diameter: f32, spacing: f32, margin: f32) -> usize {
        let radius = (diameter * 0.5).clamp(0.25, 100.0);
        let spacing = spacing.clamp(radius * 2.0 + 0.2, 400.0);
        let margin = margin.clamp(0.0, 100.0);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let mesh = object.local_mesh();
            let (min, max) = mesh.bounds();
            let mut centers = Vec::new();
            let mut z = min.z + margin + radius;
            let mut row = 0;
            while z <= max.z - margin - radius + 1e-3 && centers.len() < 48 {
                let shift = if row % 2 == 1 { spacing * 0.5 } else { 0.0 };
                let mut x = min.x + margin + radius + shift;
                while x <= max.x - margin - radius + 1e-3 && centers.len() < 48 {
                    centers.push((x, z));
                    x += spacing;
                }
                z += spacing * 0.866;
                row += 1;
            }
            let Some(drilled) = drill_vertical_holes(&mesh, &centers, radius) else {
                continue;
            };
            if is_closed(&drilled) {
                self.reseat_mesh(id, drilled);
                done += 1;
            }
        }
        done
    }

    /// A cookie cutter around the selection's outline: a wall, a cutting edge, and a rim. One new body, one undo step.
    pub fn cookie_cutter(&mut self, wall: f32, height: f32, rim: f32, rim_height: f32, edge: f32, mirror: bool) -> usize {
        let wall = wall.clamp(0.4, 10.0);
        let height = height.clamp(2.0, 100.0);
        let rim = rim.clamp(0.5, 30.0);
        let rim_height = rim_height.min(height).max(0.4);
        let edge = if edge >= wall { 0.0 } else { edge.clamp(0.0, wall) };
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut made = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let mesh = object.local_mesh();
            let mut points: Vec<(f32, f32)> = mesh.positions.iter().map(|p| (p[0], p[2])).collect();
            if mirror {
                for point in &mut points {
                    point.0 = -point.0;
                }
            }
            let hull = convex_hull(&points);
            if hull.len() < 3 {
                continue;
            }
            let Some(cutter) = cookie_mesh(&hull, wall, height, rim, rim_height, edge) else {
                continue;
            };
            let position = object.position;
            self.allocate("Cookie cutter".into(), ObjectKind::Mesh { mesh: cutter }, position);
            made += 1;
        }
        made
    }

    /// Move each surface along its outward normal. A positive distance grows the body.
    pub fn offset_selection(&mut self, distance: f32) -> usize {
        if distance.abs() < 1e-4 {
            return 0;
        }
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let mut mesh = object.local_mesh();
            if mesh.positions.is_empty() {
                continue;
            }
            let sign = if crate::mesh_volume(&mesh) < 0.0 { -1.0 } else { 1.0 };
            recompute_normals(&mut mesh);
            for (point, normal) in mesh.positions.iter_mut().zip(mesh.normals.iter()) {
                let n = Vec3::from_array(*normal) * sign;
                let moved = Vec3::from_array(*point) + n * distance;
                *point = moved.to_array();
            }
            self.reseat_mesh(id, mesh);
            done += 1;
        }
        done
    }

    /// Drop triangles that lie on the same face as `point` with `normal`.
    pub fn delete_face(&mut self, id: ObjectId, point: Vec3, normal: Vec3) -> bool {
        self.edit_face(id, point, normal, None)
    }

    /// Move that face along its normal by `distance`.
    pub fn push_face(&mut self, id: ObjectId, point: Vec3, normal: Vec3, distance: f32) -> bool {
        self.edit_face(id, point, normal, Some(distance))
    }

    fn edit_face(&mut self, id: ObjectId, point: Vec3, normal: Vec3, push: Option<f32>) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if object.locked {
            return false;
        }
        let model = object.model_matrix();
        let mut mesh = object.world_mesh();
        let n = normal.normalize_or_zero();
        if n.length_squared() < 0.5 {
            return false;
        }
        let plane = n.dot(point);
        let mut keep = Vec::new();
        let mut moved = false;
        for tri in mesh.indices.chunks_exact(3) {
            let a = Vec3::from_array(mesh.positions[tri[0] as usize]);
            let b = Vec3::from_array(mesh.positions[tri[1] as usize]);
            let c = Vec3::from_array(mesh.positions[tri[2] as usize]);
            let face = (b - a).cross(c - a).normalize_or_zero();
            let centroid = (a + b + c) / 3.0;
            let on_face = face.dot(n).abs() > 0.98 && (n.dot(centroid) - plane).abs() < 0.4;
            if on_face {
                moved = true;
                if push.is_some() {
                    keep.extend_from_slice(tri);
                }
            } else {
                keep.extend_from_slice(tri);
            }
        }
        if !moved {
            return false;
        }
        if let Some(distance) = push {
            mesh.indices = keep;
            for position in &mut mesh.positions {
                let p = Vec3::from_array(*position);
                if (n.dot(p) - plane).abs() < 0.4 {
                    *position = (p + n * distance).to_array();
                }
            }
        } else {
            mesh.indices = keep;
        }
        let _ = model;
        self.reseat_mesh(id, mesh);
        true
    }

    /// Weld corners that sit on each other and drop the smallest triangles until about `keep_ratio` remain.
    pub fn simplify_selection(&mut self, keep_ratio: f32) -> usize {
        let keep_ratio = keep_ratio.clamp(0.05, 1.0);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let mesh = object.local_mesh();
            let before = mesh.triangle_count();
            if before < 8 {
                continue;
            }
            let target = ((before as f32) * keep_ratio).round().max(4.0) as usize;
            let mesh = simplify_meshopt(&mesh, target);
            if mesh.triangle_count() >= before {
                continue;
            }
            self.reseat_mesh(id, mesh);
            done += 1;
        }
        done
    }

    fn reseat_mesh(&mut self, id: ObjectId, mut mesh: Mesh) {
        let (min, max) = mesh.bounds();
        let center_x = (min.x + max.x) * 0.5;
        let center_z = (min.z + max.z) * 0.5;
        for point in &mut mesh.positions {
            point[0] -= center_x;
            point[1] -= min.y;
            point[2] -= center_z;
        }
        recompute_normals(&mut mesh);
        if let Some(object) = self.object_mut(id) {
            object.kind = ObjectKind::Mesh { mesh };
            object.position = Vec3::new(center_x, min.y, center_z);
            object.rotation = Vec3::ZERO;
            object.revision = object.revision.saturating_add(1);
        }
    }
}

fn union_bounds(doc: &Document, ids: &[ObjectId]) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    for id in ids {
        let Some(object) = doc.object(*id) else {
            continue;
        };
        let mesh = object.world_mesh();
        if mesh.positions.is_empty() {
            continue;
        }
        let (lo, hi) = mesh.bounds();
        min = min.min(lo);
        max = max.max(hi);
        any = true;
    }
    any.then_some((min, max))
}

fn turn_xz(x: f32, z: f32, cx: f32, cz: f32, degrees: f32) -> (f32, f32) {
    let (cos, sin) = (degrees.to_radians().cos(), degrees.to_radians().sin());
    let center_y = -cz;
    let rel_x = x - cx;
    let rel_y = -z - center_y;
    let next_x = rel_x * cos - rel_y * sin;
    let next_y = rel_x * sin + rel_y * cos;
    (cx + next_x, -(center_y + next_y))
}

fn decimate(mesh: &Mesh, target: usize) -> Mesh {
    let (min, max) = mesh.bounds();
    let cell = ((max - min).length() / (target as f32).sqrt().max(1.0)).max(0.05);
    let mut index_of: std::collections::HashMap<(i32, i32, i32), u32> = std::collections::HashMap::new();
    let mut positions = Vec::new();
    let mut remap = Vec::with_capacity(mesh.positions.len());
    for point in &mesh.positions {
        let key = (
            (point[0] / cell).round() as i32,
            (point[1] / cell).round() as i32,
            (point[2] / cell).round() as i32,
        );
        let id = *index_of.entry(key).or_insert_with(|| {
            let id = positions.len() as u32;
            positions.push(*point);
            id
        });
        remap.push(id);
    }
    let mut indices = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let a = remap[tri[0] as usize];
        let b = remap[tri[1] as usize];
        let c = remap[tri[2] as usize];
        if a != b && b != c && a != c {
            indices.extend_from_slice(&[a, b, c]);
        }
    }
    let mut mesh = Mesh { positions, normals: Vec::new(), indices };
    recompute_normals(&mut mesh);
    mesh
}

fn axis_index(axis: AlignAxis) -> usize {
    match axis {
        AlignAxis::X => 0,
        AlignAxis::Y => 1,
        AlignAxis::Z => 2,
    }
}

fn recompute_normals(mesh: &mut Mesh) {
    if mesh.normals.len() != mesh.positions.len() {
        mesh.normals.resize(mesh.positions.len(), [0.0, 1.0, 0.0]);
    }
    for normal in &mut mesh.normals {
        *normal = [0.0, 0.0, 0.0];
    }
    for tri in mesh.indices.chunks_exact(3) {
        let a = Vec3::from_array(mesh.positions[tri[0] as usize]);
        let b = Vec3::from_array(mesh.positions[tri[1] as usize]);
        let c = Vec3::from_array(mesh.positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        for index in tri {
            let slot = &mut mesh.normals[*index as usize];
            *slot = (Vec3::from_array(*slot) + n).to_array();
        }
    }
    for normal in &mut mesh.normals {
        let n = Vec3::from_array(*normal).normalize_or_zero();
        *normal = if n.length_squared() > 0.0 { n.to_array() } else { [0.0, 1.0, 0.0] };
    }
}

/// Cut vertical round holes through a body that has a flat top and bottom. The result is one closed mesh.
fn drill_vertical_holes(mesh: &Mesh, centers: &[(f32, f32)], radius: f32) -> Option<Mesh> {
    if centers.is_empty() || radius < 0.05 {
        return None;
    }
    let source_volume = mesh_volume(mesh);
    let (min, max) = mesh.bounds();
    let y0 = min.y;
    let y1 = max.y;
    if y1 - y0 < 0.2 {
        return None;
    }
    let mid = (y0 + y1) * 0.5;
    let mut top_edges: HashMap<(i32, i32, i32, i32), ((f32, f32), (f32, f32))> = HashMap::new();
    let q = |x: f32, z: f32| ((x * 1000.0).round() as i32, (z * 1000.0).round() as i32);
    let mut kept = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let pts = [
            Vec3::from_array(mesh.positions[tri[0] as usize]),
            Vec3::from_array(mesh.positions[tri[1] as usize]),
            Vec3::from_array(mesh.positions[tri[2] as usize]),
        ];
        let normal = (pts[1] - pts[0]).cross(pts[2] - pts[0]);
        let flat = normal.y.abs() > normal.x.abs() && normal.y.abs() > normal.z.abs();
        let high = pts.iter().all(|point| point.y > mid);
        let low = pts.iter().all(|point| point.y < mid);
        if flat && (high || low) {
            if high {
                for i in 0..3 {
                    let a = (pts[i].x, pts[i].z);
                    let b = (pts[(i + 1) % 3].x, pts[(i + 1) % 3].z);
                    let ka = q(a.0, a.1);
                    let kb = q(b.0, b.1);
                    if ka == kb {
                        continue;
                    }
                    let key = if ka <= kb { (ka.0, ka.1, kb.0, kb.1) } else { (kb.0, kb.1, ka.0, ka.1) };
                    if top_edges.remove(&key).is_none() {
                        top_edges.insert(key, (a, b));
                    }
                }
            }
            continue;
        }
        kept.push([pts[0], pts[1], pts[2]]);
    }
    let mut outer = chain_flat(&top_edges.into_values().collect::<Vec<_>>());
    if outer.len() < 3 {
        return None;
    }
    if signed_area(&outer) < 0.0 {
        outer.reverse();
    }
    let segments = 16;
    let mut holes = Vec::new();
    for (x, z) in centers {
        let mut hole = Vec::with_capacity(segments);
        for step in 0..segments {
            let angle = step as f32 / segments as f32 * std::f32::consts::TAU;
            hole.push((x + radius * angle.cos(), z + radius * angle.sin()));
        }
        if signed_area(&hole) > 0.0 {
            hole.reverse();
        }
        holes.push(hole);
    }
    let Some(top) = triangulate_with_holes(&outer, &holes) else {
        eprintln!("earcut none outer {}", outer.len());
        return None;
    };
    if centers.len() > 1 {
        let mut area = 0.0;
        for tri in &top {
            area += tri[0].0 * (tri[1].1 - tri[2].1) + tri[1].0 * (tri[2].1 - tri[0].1) + tri[2].0 * (tri[0].1 - tri[1].1);
        }
        eprintln!("outer {outer:?} area {} tri {}", signed_area(&outer), area * 0.5);
    }
    let mut tris = kept;
    for tri in &top {
        let (a, b, c) = (tri[0], tri[1], tri[2]);
        // This port's solids wind inwards, so the top normal points down.
        tris.push([Vec3::new(a.0, y1, a.1), Vec3::new(c.0, y1, c.1), Vec3::new(b.0, y1, b.1)]);
        tris.push([Vec3::new(a.0, y0, a.1), Vec3::new(b.0, y0, b.1), Vec3::new(c.0, y0, c.1)]);
    }
    for hole in &holes {
        for index in 0..hole.len() {
            let (x0, z0) = hole[index];
            let (x1, z1) = hole[(index + 1) % hole.len()];
            let b0 = Vec3::new(x0, y0, z0);
            let b1 = Vec3::new(x1, y0, z1);
            let t1 = Vec3::new(x1, y1, z1);
            let t0 = Vec3::new(x0, y1, z0);
            tris.push([b0, t1, t0]);
            tris.push([b0, b1, t1]);
        }
    }
    let mut mesh = mesh_from_tris(tris);
    orient_mesh(&mut mesh);
    if mesh_volume(&mesh) * source_volume < 0.0 {
        for tri in mesh.indices.chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
    }
    is_closed(&mesh).then_some(mesh)
}

fn orient_mesh(mesh: &mut Mesh) {
    let key = |p: [f32; 3]| [(p[0] * 1000.0).round() as i32, (p[1] * 1000.0).round() as i32, (p[2] * 1000.0).round() as i32];
    let corners = |mesh: &Mesh, tri: usize| {
        [
            key(mesh.positions[mesh.indices[tri * 3] as usize]),
            key(mesh.positions[mesh.indices[tri * 3 + 1] as usize]),
            key(mesh.positions[mesh.indices[tri * 3 + 2] as usize]),
        ]
    };
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 {
        return;
    }
    let undirected = |a, b| if a <= b { (a, b) } else { (b, a) };
    let mut edges: HashMap<([i32; 3], [i32; 3]), Vec<usize>> = HashMap::new();
    for tri in 0..tri_count {
        let pts = corners(mesh, tri);
        for i in 0..3 {
            edges.entry(undirected(pts[i], pts[(i + 1) % 3])).or_default().push(tri);
        }
    }
    let has_dir = |mesh: &Mesh, tri: usize, a, b| {
        let pts = corners(mesh, tri);
        (0..3).any(|i| pts[i] == a && pts[(i + 1) % 3] == b)
    };
    let mut seen = vec![false; tri_count];
    let mut queue = vec![0usize];
    seen[0] = true;
    while let Some(tri) = queue.pop() {
        let pts = corners(mesh, tri);
        for i in 0..3 {
            let Some(neighbors) = edges.get(&undirected(pts[i], pts[(i + 1) % 3])) else {
                continue;
            };
            for &neighbor in neighbors.clone().iter() {
                if neighbor == tri || seen[neighbor] {
                    continue;
                }
                if has_dir(mesh, neighbor, pts[i], pts[(i + 1) % 3]) {
                    mesh.indices.swap(neighbor * 3 + 1, neighbor * 3 + 2);
                }
                seen[neighbor] = true;
                queue.push(neighbor);
            }
        }
    }
}

fn signed_area(loop_pts: &[(f32, f32)]) -> f32 {
    let mut area = 0.0;
    for index in 0..loop_pts.len() {
        let a = loop_pts[index];
        let b = loop_pts[(index + 1) % loop_pts.len()];
        area += a.0 * b.1 - b.0 * a.1;
    }
    area * 0.5
}

fn chain_flat(segments: &[((f32, f32), (f32, f32))]) -> Vec<(f32, f32)> {
    let key = |p: (f32, f32)| ((p.0 * 1000.0).round() as i32, (p.1 * 1000.0).round() as i32);
    let mut next: HashMap<(i32, i32), Vec<(f32, f32)>> = HashMap::new();
    for (a, b) in segments {
        next.entry(key(*a)).or_default().push(*b);
        next.entry(key(*b)).or_default().push(*a);
    }
    if segments.is_empty() {
        return Vec::new();
    }
    let (start, mut cur) = segments[0];
    let mut prev = start;
    let mut loop_pts = vec![start];
    for _ in 0..segments.len() + 2 {
        loop_pts.push(cur);
        if key(cur) == key(start) {
            loop_pts.pop();
            return loop_pts;
        }
        let Some(options) = next.get(&key(cur)) else {
            return Vec::new();
        };
        let mut found = None;
        for cand in options {
            if key(*cand) != key(prev) {
                found = Some(*cand);
                break;
            }
        }
        let Some(cand) = found else {
            return Vec::new();
        };
        prev = cur;
        cur = cand;
    }
    Vec::new()
}

fn triangulate_with_holes(outer: &[(f32, f32)], holes: &[Vec<(f32, f32)>]) -> Option<Vec<[(f32, f32); 3]>> {
    let mut data = Vec::new();
    let mut hole_at = Vec::new();
    for point in outer {
        data.push(point.0 as f64);
        data.push(point.1 as f64);
    }
    for hole in holes {
        if hole.len() < 3 {
            continue;
        }
        hole_at.push(data.len() / 2);
        for point in hole {
            data.push(point.0 as f64);
            data.push(point.1 as f64);
        }
    }
    let mut indices = earcutr::earcut(&data, &hole_at, 2).ok()?;
    let holes_used = hole_at.first().is_none_or(|start| indices.iter().any(|index| *index >= *start));
    if !holes_used {
        let spliced = splice_holes(outer, holes);
        data = spliced.iter().flat_map(|point| [point.0 as f64, point.1 as f64]).collect();
        indices = earcutr::earcut(&data, &[], 2).ok()?;
    }
    if indices.len() < 3 {
        return None;
    }
    let mut tris = Vec::new();
    for tri in indices.chunks_exact(3) {
        let point = |index: usize| {
            let slot = index * 2;
            (data[slot] as f32, data[slot + 1] as f32)
        };
        tris.push([point(tri[0]), point(tri[1]), point(tri[2])]);
    }
    Some(tris)
}

fn splice_holes(outer: &[(f32, f32)], holes: &[Vec<(f32, f32)>]) -> Vec<(f32, f32)> {
    let mut poly = outer.to_vec();
    for hole in holes {
        if hole.len() < 3 || poly.is_empty() {
            continue;
        }
        let (hole_at, _) = hole
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.0.partial_cmp(&right.1.0).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let mut best = 0;
        let mut best_d = f32::MAX;
        for (index, point) in poly.iter().enumerate() {
            let d = (point.0 - hole[hole_at].0).hypot(point.1 - hole[hole_at].1);
            if d < best_d {
                best_d = d;
                best = index;
            }
        }
        let mut next = Vec::with_capacity(poly.len() + hole.len() + 2);
        next.extend_from_slice(&poly[..=best]);
        for step in 0..=hole.len() {
            next.push(hole[(hole_at + step) % hole.len()]);
        }
        next.push(poly[best]);
        next.extend_from_slice(&poly[best + 1..]);
        poly = next;
    }
    poly
}

fn chamfer_sharp(mesh: &Mesh, distance: f32) -> Option<Mesh> {
    let planes = chamfer_planes(mesh, distance);
    if planes.is_empty() {
        return None;
    }
    let mut current = mesh.clone();
    let mut cut = 0;
    for (normal, origin) in planes {
        let (keep, _) = clip_plane(&current, normal, origin);
        if keep.triangle_count() < 4 || !is_closed(&keep) {
            continue;
        }
        let before = mesh_volume(&current).abs();
        let after = mesh_volume(&keep).abs();
        if after >= before * 0.995 || after < before * 0.25 {
            continue;
        }
        current = keep;
        cut += 1;
    }
    (cut > 0).then_some(current)
}

fn chamfer_planes(mesh: &Mesh, distance: f32) -> Vec<(Vec3, Vec3)> {
    let mut edges: HashMap<(i32, i32, i32, i32, i32, i32), (Vec3, Vec3, Vec<Vec3>)> = HashMap::new();
    let key = |p: Vec3| ((p.x * 1000.0).round() as i32, (p.y * 1000.0).round() as i32, (p.z * 1000.0).round() as i32);
    for tri in mesh.indices.chunks_exact(3) {
        let pts = [
            Vec3::from_array(mesh.positions[tri[0] as usize]),
            Vec3::from_array(mesh.positions[tri[1] as usize]),
            Vec3::from_array(mesh.positions[tri[2] as usize]),
        ];
        let mut normal = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalize_or_zero();
        if normal.length_squared() < 0.5 {
            continue;
        }
        let (min, max) = mesh.bounds();
        let centre = (min + max) * 0.5;
        if normal.dot(pts[0] - centre) < 0.0 {
            normal = -normal;
        }
        for i in 0..3 {
            let a = pts[i];
            let b = pts[(i + 1) % 3];
            let mut ka = key(a);
            let mut kb = key(b);
            let (pa, pb) = if ka <= kb { (a, b) } else { std::mem::swap(&mut ka, &mut kb); (b, a) };
            let slot = edges.entry((ka.0, ka.1, ka.2, kb.0, kb.1, kb.2)).or_insert((pa, pb, Vec::new()));
            slot.2.push(normal);
        }
    }
    let mut planes = Vec::new();
    for (_, (a, b, normals)) in edges {
        if normals.len() != 2 {
            continue;
        }
        let n1 = normals[0];
        let n2 = normals[1];
        if n1.dot(n2) > 0.85 {
            continue;
        }
        let length = (b - a).length();
        if length < distance * 2.2 {
            continue;
        }
        let dir = (b - a).normalize_or_zero();
        let outward = (n1 + n2).normalize_or_zero();
        if outward.length_squared() < 0.5 {
            continue;
        }
        let mid = (a + b) * 0.5;
        let mut into1 = dir.cross(n1).normalize_or_zero();
        if into1.dot(n2) > 0.0 {
            into1 = -into1;
        }
        let mut into2 = dir.cross(n2).normalize_or_zero();
        if into2.dot(n1) > 0.0 {
            into2 = -into2;
        }
        if into1.length_squared() < 0.5 || into2.length_squared() < 0.5 {
            continue;
        }
        let p1 = mid + into1 * distance;
        let p2 = mid + into2 * distance;
        let mut normal = (p2 - p1).cross(dir).normalize_or_zero();
        if normal.length_squared() < 0.5 {
            continue;
        }
        if normal.dot(outward) < 0.0 {
            normal = -normal;
        }
        planes.push((normal, p1));
        if planes.len() >= 48 {
            break;
        }
    }
    planes
}

fn simplify_meshopt(mesh: &Mesh, target_triangles: usize) -> Mesh {
    if mesh.indices.len() < 12 || mesh.positions.is_empty() {
        return mesh.clone();
    }
    let target = (target_triangles * 3).clamp(12, mesh.indices.len());
    let mut destination = vec![0_u32; mesh.indices.len()];
    let kept = meshopt_rs::simplify::simplify(
        &mut destination,
        &mesh.indices,
        &mesh.positions,
        target,
        1.0,
    );
    if kept < 12 || kept >= mesh.indices.len() {
        return decimate(mesh, target_triangles);
    }
    destination.truncate(kept);
    let mut next = Mesh {
        positions: mesh.positions.clone(),
        normals: Vec::new(),
        indices: destination,
    };
    recompute_normals(&mut next);
    next
}

fn chamfered_cylinder(radius: f32, height: f32, segments: u32, chamfer: f32) -> Mesh {
    let segments = segments.max(3);
    let mut tris = Vec::new();
    let ring = |y: f32, r: f32, i: u32| {
        let angle = i as f32 / segments as f32 * std::f32::consts::TAU;
        Vec3::new(angle.cos() * r, y, angle.sin() * r)
    };
    let rings = [(0.0, radius - chamfer), (chamfer, radius), (height - chamfer, radius), (height, radius - chamfer)];
    for index in 0..segments {
        let next = (index + 1) % segments;
        for pair in rings.windows(2) {
            let a0 = ring(pair[0].0, pair[0].1, index);
            let a1 = ring(pair[0].0, pair[0].1, next);
            let b1 = ring(pair[1].0, pair[1].1, next);
            let b0 = ring(pair[1].0, pair[1].1, index);
            tris.push([a0, a1, b1]);
            tris.push([a0, b1, b0]);
        }
        let top = Vec3::new(0.0, height, 0.0);
        tris.push([top, ring(height, radius - chamfer, index), ring(height, radius - chamfer, next)]);
        let bottom = Vec3::new(0.0, 0.0, 0.0);
        tris.push([bottom, ring(0.0, radius - chamfer, next), ring(0.0, radius - chamfer, index)]);
    }
    mesh_from_tris(tris)
}

/// Cut `mesh` with a plane through `origin` and keep both sides, capping each cut so the pieces are closed.
fn clip_plane(mesh: &Mesh, normal: Vec3, origin: Vec3) -> (Mesh, Mesh) {
    let normal = normal.normalize_or_zero();
    let mut low = Vec::new();
    let mut high = Vec::new();
    let mut segments = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let points = [
            Vec3::from_array(mesh.positions[tri[0] as usize]),
            Vec3::from_array(mesh.positions[tri[1] as usize]),
            Vec3::from_array(mesh.positions[tri[2] as usize]),
        ];
        let mut neg = Vec::new();
        let mut pos = Vec::new();
        let mut hits = Vec::new();
        for i in 0..3 {
            let a = points[i];
            let b = points[(i + 1) % 3];
            let da = normal.dot(a - origin);
            let db = normal.dot(b - origin);
            if da <= 1e-5 {
                neg.push(a);
            }
            if da >= -1e-5 {
                pos.push(a);
            }
            if da * db < 0.0 {
                let t = da / (da - db);
                let hit = a + (b - a) * t;
                neg.push(hit);
                pos.push(hit);
                hits.push(hit);
            }
        }
        if hits.len() == 2 {
            segments.push((hits[0], hits[1]));
        }
        fan(&mut low, &neg);
        fan(&mut high, &pos);
    }
    for loop_pts in chain_loops(&segments) {
        low.extend(cap_fan(&loop_pts, -normal));
        high.extend(cap_fan(&loop_pts, normal));
    }
    (mesh_from_tris(low), mesh_from_tris(high))
}

fn chain_loops(segments: &[(Vec3, Vec3)]) -> Vec<Vec<Vec3>> {
    let key = |p: Vec3| ((p.x * 1.0e4).round() as i32, (p.y * 1.0e4).round() as i32, (p.z * 1.0e4).round() as i32);
    let edge_key = |a, b| if a <= b { (a, b) } else { (b, a) };
    let mut next: HashMap<(i32, i32, i32), Vec<Vec3>> = HashMap::new();
    for (a, b) in segments {
        next.entry(key(*a)).or_default().push(*b);
        next.entry(key(*b)).or_default().push(*a);
    }
    let mut used = HashSet::new();
    let mut loops = Vec::new();
    for (start, other) in segments {
        let first = edge_key(key(*start), key(*other));
        if !used.insert(first) {
            continue;
        }
        let mut loop_pts = vec![*start];
        let mut prev = *start;
        let mut cur = *other;
        for _ in 0..segments.len() + 2 {
            loop_pts.push(cur);
            if key(cur) == key(*start) {
                break;
            }
            let Some(options) = next.get(&key(cur)) else {
                break;
            };
            let mut found = None;
            for cand in options {
                if key(*cand) == key(prev) {
                    continue;
                }
                let mark = edge_key(key(cur), key(*cand));
                if used.contains(&mark) {
                    continue;
                }
                found = Some((*cand, mark));
                break;
            }
            let Some((cand, mark)) = found else {
                break;
            };
            used.insert(mark);
            prev = cur;
            cur = cand;
        }
        if loop_pts.len() >= 3 && key(*loop_pts.last().unwrap()) == key(*start) {
            loop_pts.pop();
            loops.push(loop_pts);
        }
    }
    loops
}

fn cap_fan(loop_pts: &[Vec3], outward: Vec3) -> Vec<[Vec3; 3]> {
    if loop_pts.len() < 3 {
        return Vec::new();
    }
    let center = loop_pts.iter().copied().fold(Vec3::ZERO, |sum, point| sum + point) / loop_pts.len() as f32;
    let mut tris = Vec::new();
    for index in 0..loop_pts.len() {
        let a = loop_pts[index];
        let b = loop_pts[(index + 1) % loop_pts.len()];
        let winding = (a - center).cross(b - center);
        if winding.dot(outward) >= 0.0 {
            tris.push([center, a, b]);
        } else {
            tris.push([center, b, a]);
        }
    }
    tris
}

fn fan(out: &mut Vec<[Vec3; 3]>, poly: &[Vec3]) {
    if poly.len() < 3 {
        return;
    }
    for i in 1..poly.len() - 1 {
        out.push([poly[0], poly[i], poly[i + 1]]);
    }
}

fn mesh_from_tris(tris: Vec<[Vec3; 3]>) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for tri in tris {
        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
        let normal = if n.length_squared() > 0.0 { n } else { Vec3::Y };
        let base = positions.len() as u32;
        for point in tri {
            positions.push(point.to_array());
            normals.push(normal.to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Mesh { positions, normals, indices }
}

fn cookie_mesh(hull: &[(f32, f32)], wall: f32, height: f32, rim: f32, rim_height: f32, edge: f32) -> Option<Mesh> {
    let inner = hull.to_vec();
    let wall_outer = offset_polygon(&inner, wall);
    let rim_outer = offset_polygon(&inner, wall + rim);
    let lip = offset_polygon(&inner, (wall - edge).max(0.2));
    if wall_outer.len() != inner.len() || rim_outer.len() != inner.len() || lip.len() != inner.len() {
        return None;
    }
    let mut tris = Vec::new();
    annulus(&mut tris, &rim_outer, &inner, 0.0, true);
    band(&mut tris, &rim_outer, 0.0, rim_height, true);
    annulus(&mut tris, &wall_outer, &rim_outer, rim_height, true);
    band(&mut tris, &wall_outer, rim_height, height, true);
    if edge > 0.05 {
        band(&mut tris, &lip, height, height, true);
        quad_ring(&mut tris, &wall_outer, height, &lip, height);
    }
    annulus(&mut tris, &inner, &lip, height, true);
    band(&mut tris, &inner, height, 0.0, false);
    let mut mesh = mesh_from_tris(tris);
    orient_mesh(&mut mesh);
    if mesh_volume(&mesh) > 0.0 {
        for tri in mesh.indices.chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
    }
    is_closed(&mesh).then_some(mesh)
}

fn convex_hull(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4);
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut lower = Vec::new();
    for point in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0 {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0 {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn offset_polygon(poly: &[(f32, f32)], dist: f32) -> Vec<(f32, f32)> {
    let n = poly.len();
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        let prev = poly[(index + n - 1) % n];
        let cur = poly[index];
        let next = poly[(index + 1) % n];
        let n1 = right_normal(prev, cur);
        let n2 = right_normal(cur, next);
        let bis = (n1 + n2).normalize_or_zero();
        if bis.length_squared() < 0.5 {
            out.push((cur.0 + n1.x * dist, cur.1 + n1.z * dist));
            continue;
        }
        let scale = dist / bis.dot(n1).max(0.25);
        out.push((cur.0 + bis.x * scale, cur.1 + bis.z * scale));
    }
    out
}

fn right_normal(a: (f32, f32), b: (f32, f32)) -> Vec3 {
    let dx = b.0 - a.0;
    let dz = b.1 - a.1;
    Vec3::new(dz, 0.0, -dx).normalize_or_zero()
}

fn annulus(tris: &mut Vec<[Vec3; 3]>, outer: &[(f32, f32)], inner: &[(f32, f32)], y: f32, up: bool) {
    for index in 0..outer.len() {
        let next = (index + 1) % outer.len();
        let o0 = Vec3::new(outer[index].0, y, outer[index].1);
        let o1 = Vec3::new(outer[next].0, y, outer[next].1);
        let i0 = Vec3::new(inner[index].0, y, inner[index].1);
        let i1 = Vec3::new(inner[next].0, y, inner[next].1);
        if up {
            tris.push([o0, i0, i1]);
            tris.push([o0, i1, o1]);
        } else {
            tris.push([o0, i1, i0]);
            tris.push([o0, o1, i1]);
        }
    }
}

fn band(tris: &mut Vec<[Vec3; 3]>, ring: &[(f32, f32)], y0: f32, y1: f32, outward: bool) {
    if (y1 - y0).abs() < 1e-4 {
        return;
    }
    for index in 0..ring.len() {
        let next = (index + 1) % ring.len();
        let a0 = Vec3::new(ring[index].0, y0, ring[index].1);
        let a1 = Vec3::new(ring[next].0, y0, ring[next].1);
        let b1 = Vec3::new(ring[next].0, y1, ring[next].1);
        let b0 = Vec3::new(ring[index].0, y1, ring[index].1);
        if outward {
            tris.push([a0, a1, b1]);
            tris.push([a0, b1, b0]);
        } else {
            tris.push([a0, b1, a1]);
            tris.push([a0, b0, b1]);
        }
    }
}

fn quad_ring(tris: &mut Vec<[Vec3; 3]>, a: &[(f32, f32)], ya: f32, b: &[(f32, f32)], yb: f32) {
    for index in 0..a.len() {
        let next = (index + 1) % a.len();
        let a0 = Vec3::new(a[index].0, ya, a[index].1);
        let a1 = Vec3::new(a[next].0, ya, a[next].1);
        let b1 = Vec3::new(b[next].0, yb, b[next].1);
        let b0 = Vec3::new(b[index].0, yb, b[index].1);
        tris.push([a0, b0, b1]);
        tris.push([a0, b1, a1]);
    }
}

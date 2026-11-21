//! Mirror, pattern, chamfer, hollow, and split. Each edits the selection in one step.

use glam::Vec3;

use crate::boolean::{boolean, BooleanOp};
use crate::document::{Document, ObjectId, ObjectKind};
use crate::mesh::Mesh;
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

    /// Bevel the top edges of each selected box by `distance` millimetres.
    pub fn chamfer_selection(&mut self, distance: f32) -> usize {
        let distance = distance.max(0.05);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some((width, depth, height)) = self.object(id).and_then(|object| match &object.kind {
                ObjectKind::Box { width, depth, height } if !object.locked => Some((*width, *depth, *height)),
                _ => None,
            }) else {
                continue;
            };
            let limit = width.min(depth).min(height) * 0.45;
            let distance = distance.min(limit);
            if distance < 0.05 {
                continue;
            }
            let mesh = chamfered_box(width, depth, height, distance);
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
            let (low, high) = clip_mesh(&mesh, axis_index(axis), plane);
            if low.triangle_count() == 0 || high.triangle_count() == 0 {
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

    /// Round holes through the top of each selected body. Defaults match the live pattern: 6 mm, 9 mm apart, 4 mm from the edge.
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
            let model = object.model_matrix();
            let mut holes = 0;
            let mut z = min.z + margin + radius;
            let mut row = 0;
            while z <= max.z - margin - radius + 1e-3 && holes < 80 {
                let shift = if row % 2 == 1 { spacing * 0.5 } else { 0.0 };
                let mut x = min.x + margin + radius + shift;
                while x <= max.x - margin - radius + 1e-3 && holes < 80 {
                    let local = Vec3::new(x, min.y, z);
                    let world = model.transform_point3(local);
                    let cutter = self.allocate(
                        format!("Hole {}", holes + 1),
                        ObjectKind::Cylinder {
                            radius,
                            height: max.y - min.y,
                            segments: 16,
                        },
                        world,
                    );
                    if let Some(object) = self.object_mut(cutter) {
                        object.hole = true;
                    }
                    holes += 1;
                    x += spacing;
                }
                z += spacing * 0.866;
                row += 1;
            }
            if holes > 0 {
                done += 1;
            }
        }
        done
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
            let mesh = decimate(&mesh, target);
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

fn chamfered_box(width: f32, depth: f32, height: f32, chamfer: f32) -> Mesh {
    let hx = width * 0.5;
    let hz = depth * 0.5;
    let y_lip = height - chamfer;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut push = |p: [f32; 3]| -> u32 {
        let id = positions.len() as u32;
        positions.push(p);
        normals.push([0.0, 1.0, 0.0]);
        id
    };
    let quad = |indices: &mut Vec<u32>, a, b, c, d| {
        indices.extend_from_slice(&[a, b, c, a, c, d]);
    };
    // Bottom.
    let b00 = push([-hx, 0.0, -hz]);
    let b10 = push([hx, 0.0, -hz]);
    let b11 = push([hx, 0.0, hz]);
    let b01 = push([-hx, 0.0, hz]);
    quad(&mut indices, b00, b01, b11, b10);
    // Sides up to the lip.
    let l00 = push([-hx, y_lip, -hz]);
    let l10 = push([hx, y_lip, -hz]);
    let l11 = push([hx, y_lip, hz]);
    let l01 = push([-hx, y_lip, hz]);
    quad(&mut indices, b00, b10, l10, l00);
    quad(&mut indices, b10, b11, l11, l10);
    quad(&mut indices, b11, b01, l01, l11);
    quad(&mut indices, b01, b00, l00, l01);
    // Top inset and the four bevels.
    let t00 = push([-(hx - chamfer), height, -(hz - chamfer)]);
    let t10 = push([hx - chamfer, height, -(hz - chamfer)]);
    let t11 = push([hx - chamfer, height, hz - chamfer]);
    let t01 = push([-(hx - chamfer), height, hz - chamfer]);
    quad(&mut indices, l00, l10, t10, t00);
    quad(&mut indices, l10, l11, t11, t10);
    quad(&mut indices, l11, l01, t01, t11);
    quad(&mut indices, l01, l00, t00, t01);
    quad(&mut indices, t00, t10, t11, t01);
    let mut mesh = Mesh { positions, normals, indices };
    recompute_normals(&mut mesh);
    mesh
}

fn clip_mesh(mesh: &Mesh, axis: usize, plane: f32) -> (Mesh, Mesh) {
    let mut low = Vec::new();
    let mut high = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let points = [
            Vec3::from_array(mesh.positions[tri[0] as usize]),
            Vec3::from_array(mesh.positions[tri[1] as usize]),
            Vec3::from_array(mesh.positions[tri[2] as usize]),
        ];
        let mut neg = Vec::new();
        let mut pos = Vec::new();
        for i in 0..3 {
            let a = points[i];
            let b = points[(i + 1) % 3];
            let da = a[axis] - plane;
            let db = b[axis] - plane;
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
            }
        }
        fan(&mut low, &neg);
        fan(&mut high, &pos);
    }
    (mesh_from_tris(low), mesh_from_tris(high))
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

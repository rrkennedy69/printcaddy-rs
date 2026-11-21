//! Mirror, pattern, chamfer, hollow, and split. Each edits the selection in one step.

use glam::Vec3;

use crate::boolean::{boolean, BooleanOp};
use crate::document::{Document, ObjectId, ObjectKind};
use crate::mesh::{box_mesh, Mesh};
use crate::AlignAxis;

impl Document {
    /// Reflect every selected body across the centre of the selection on `axis`.
    pub fn mirror_selection(&mut self, axis: AlignAxis) -> usize {
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

    /// Hollow each selected box: walls and a floor of `wall` millimetres, open at the top.
    pub fn hollow_selection(&mut self, wall: f32) -> usize {
        let wall = wall.max(0.2);
        let ids: Vec<ObjectId> = self.selected_ids().to_vec();
        let mut done = 0;
        for id in ids {
            let Some((width, depth, height)) = self.object(id).and_then(|object| match &object.kind {
                ObjectKind::Box { width, depth, height } if !object.locked => Some((*width, *depth, *height)),
                _ => None,
            }) else {
                continue;
            };
            if wall * 2.0 >= width.min(depth) || wall >= height {
                continue;
            }
            let outer = box_mesh(width, depth, height);
            let inner = box_mesh(width - 2.0 * wall, depth - 2.0 * wall, height - wall).translated(Vec3::Y * wall);
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

    /// Cut each selected body through its centre on `axis` and keep both pieces.
    pub fn split_selection(&mut self, axis: AlignAxis) -> usize {
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
            let plane = match axis {
                AlignAxis::X => (min.x + max.x) * 0.5,
                AlignAxis::Y => (min.y + max.y) * 0.5,
                AlignAxis::Z => (min.z + max.z) * 0.5,
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

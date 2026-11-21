//! Copy placement, align, centre, drop, and lay-flat. These match the live editor's
//! geometry: paste steps by 12 mm, align matches one axis of the anchor, drop rests
//! on the plate or the solid underneath.

use std::collections::HashMap;

use glam::{Quat, Vec3};

use crate::document::{Document, Object, ObjectId, ObjectKind};
use crate::ray::Ray;
use crate::PLATE_SIZE_MM;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignTarget {
    Min,
    Center,
    Max,
}

impl AlignAxis {
    fn coord(self, min: Vec3, max: Vec3, target: AlignTarget) -> f32 {
        let (lo, hi) = match self {
            AlignAxis::X => (min.x, max.x),
            AlignAxis::Y => (min.y, max.y),
            AlignAxis::Z => (min.z, max.z),
        };
        match target {
            AlignTarget::Min => lo,
            AlignTarget::Max => hi,
            AlignTarget::Center => (lo + hi) * 0.5,
        }
    }

    fn add(self, delta: f32) -> Vec3 {
        match self {
            AlignAxis::X => Vec3::new(delta, 0.0, 0.0),
            AlignAxis::Y => Vec3::new(0.0, delta, 0.0),
            AlignAxis::Z => Vec3::new(0.0, 0.0, delta),
        }
    }
}

const EPS: f32 = 0.0005;

impl Document {
    /// Paste stored bodies. Each moves 12 mm in X and Z, and not past 10 mm outside the plate.
    pub fn place_copy(&mut self, source: &Object, position: Vec3) -> ObjectId {
        self.allocate_copy(source, position)
    }

    pub fn paste_copies(&mut self, sources: &[Object]) -> Vec<ObjectId> {
        let limit = PLATE_SIZE_MM * 0.5 + 10.0;
        let mut map: HashMap<u64, ObjectId> = HashMap::new();
        let mut order: Vec<Object> = sources.to_vec();
        order.sort_by_key(|object| object.parent.is_some());
        for source in &order {
            let position = Vec3::new(
                (source.position.x + 12.0).min(limit),
                source.position.y,
                (source.position.z + 12.0).min(limit),
            );
            let id = self.allocate_copy(source, position);
            map.insert(source.id.0, id);
            if let Some(parent) = source.parent.and_then(|parent| map.get(&parent.0).copied()) {
                if let Some(object) = self.object_mut(id) {
                    object.parent = Some(parent);
                }
            }
        }
        for source in &order {
            let Some(&id) = map.get(&source.id.0) else {
                continue;
            };
            if let ObjectKind::Group { members, .. } = &source.kind {
                let mapped: Vec<ObjectId> = members
                    .iter()
                    .filter_map(|member| map.get(&member.0).copied())
                    .collect();
                if let Some(object) = self.object_mut(id) {
                    if let ObjectKind::Group { members, .. } = &mut object.kind {
                        *members = mapped;
                    }
                }
            }
        }
        let roots: Vec<ObjectId> = order
            .iter()
            .filter(|object| object.parent.is_none())
            .filter_map(|object| map.get(&object.id.0).copied())
            .collect();
        self.replace_selection(roots.clone());
        roots
    }

    fn allocate_copy(&mut self, source: &Object, position: Vec3) -> ObjectId {
        let name = if source.parent.is_none() {
            format!("{} copy", source.name)
        } else {
            source.name.clone()
        };
        let id = self.allocate(name, source.kind.clone(), position);
        if let Some(object) = self.object_mut(id) {
            object.rotation = source.rotation;
            object.color = source.color;
            object.hole = source.hole;
            object.transparent = source.transparent;
            object.filament = source.filament.clone();
            object.locked = false;
            object.hidden = false;
        }
        id
    }

    /// Line the other selected bodies up with the anchor (the last selection) on one axis.
    pub fn align_selection(&mut self, axis: AlignAxis, target: AlignTarget) -> usize {
        let ids = self.selected_ids().to_vec();
        let Some(anchor) = ids.last().copied() else {
            return 0;
        };
        let Some(anchor_value) = self.world_bounds(anchor).map(|(min, max)| axis.coord(min, max, target)) else {
            return 0;
        };
        let mut moved = 0;
        for id in ids {
            if id == anchor {
                continue;
            }
            let Some(object) = self.object(id) else {
                continue;
            };
            if object.locked {
                continue;
            }
            let Some((min, max)) = self.world_bounds(id) else {
                continue;
            };
            let delta = anchor_value - axis.coord(min, max, target);
            if delta.abs() <= EPS {
                continue;
            }
            if let Some(object) = self.object_mut(id) {
                object.position += axis.add(delta);
                moved += 1;
            }
        }
        moved
    }

    /// Move the selection together so its footprint centre sits on the plate centre.
    pub fn center_selection(&mut self) -> bool {
        let ids: Vec<ObjectId> = self
            .selected_ids()
            .iter()
            .copied()
            .filter(|id| self.object(*id).is_some_and(|object| !object.locked))
            .collect();
        let Some((min, max)) = self.combined_bounds(&ids) else {
            return false;
        };
        let delta = Vec3::new(-(min.x + max.x) * 0.5, 0.0, -(min.z + max.z) * 0.5);
        if delta.length_squared() <= EPS * EPS {
            return false;
        }
        for id in ids {
            if let Some(object) = self.object_mut(id) {
                object.position += delta;
            }
        }
        true
    }

    /// Drop the selection onto the plate, or onto the highest solid whose footprint overlaps.
    pub fn drop_selection(&mut self) -> bool {
        let ids: Vec<ObjectId> = self
            .selected_ids()
            .iter()
            .copied()
            .filter(|id| self.object(*id).is_some_and(|object| !object.locked && !object.hidden))
            .collect();
        self.drop_ids(&ids)
    }

    fn drop_ids(&mut self, ids: &[ObjectId]) -> bool {
        let boxes: Vec<(ObjectId, Vec3, Vec3)> = ids
            .iter()
            .filter_map(|id| self.world_bounds(*id).map(|(min, max)| (*id, min, max)))
            .collect();
        if boxes.is_empty() {
            return false;
        }
        let bottom = boxes.iter().map(|(_, min, _)| min.y).fold(f32::INFINITY, f32::min);
        let mut rest = 0.0_f32;
        for other in self.objects().to_vec() {
            if ids.contains(&other.id) || other.hole || other.hidden {
                continue;
            }
            let Some((min, max)) = self.world_bounds(other.id) else {
                continue;
            };
            if max.y > bottom + EPS {
                continue;
            }
            let overlaps = boxes.iter().any(|(_, bmin, bmax)| {
                bmin.x < max.x - EPS && bmax.x > min.x + EPS && bmin.z < max.z - EPS && bmax.z > min.z + EPS
            });
            if overlaps && max.y > rest {
                rest = max.y;
            }
        }
        let delta = rest - bottom;
        if delta.abs() < EPS {
            return false;
        }
        for id in ids {
            if let Some(object) = self.object_mut(*id) {
                object.position.y += delta;
            }
        }
        true
    }

    /// Turn `id` so `world_normal` points down, then drop it.
    pub fn lay_flat(&mut self, id: ObjectId, world_normal: Vec3) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if object.locked {
            return false;
        }
        let from = world_normal.normalize_or_zero();
        if from.length_squared() < 0.5 {
            return false;
        }
        let down = Vec3::NEG_Y;
        let turn = if from.dot(down) < -1.0 + 1e-6 {
            Quat::from_axis_angle(Vec3::X, std::f32::consts::PI)
        } else {
            Quat::from_rotation_arc(from, down)
        };
        let current = Quat::from_euler(
            glam::EulerRot::XYZ,
            object.rotation.x.to_radians(),
            object.rotation.y.to_radians(),
            object.rotation.z.to_radians(),
        );
        let (x, y, z) = (turn * current).to_euler(glam::EulerRot::XYZ);
        if let Some(object) = self.object_mut(id) {
            object.rotation = Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());
            object.revision = object.revision.saturating_add(1);
        }
        self.drop_ids(&[id]);
        true
    }

    pub fn hide_ids(&mut self, ids: &[ObjectId], hidden: bool) {
        for id in ids {
            if let Some(object) = self.object_mut(*id) {
                object.hidden = hidden;
            }
        }
    }

    pub fn show_all(&mut self) {
        for object in self.objects_mut() {
            object.hidden = false;
        }
    }

    pub fn set_name(&mut self, id: ObjectId, name: String) {
        if let Some(object) = self.object_mut(id) {
            object.name = name;
        }
    }

    pub fn set_locked(&mut self, id: ObjectId, locked: bool) {
        if let Some(object) = self.object_mut(id) {
            object.locked = locked;
        }
        if locked {
            let kept: Vec<ObjectId> = self
                .selected_ids()
                .iter()
                .copied()
                .filter(|selected| *selected != id)
                .collect();
            self.replace_selection(kept);
        }
    }

    /// Point and outward normal of the triangle the ray hits first.
    pub fn face_hit(&self, id: ObjectId, ray: Ray) -> Option<(Vec3, Vec3)> {
        let mesh = self.object(id)?.world_mesh();
        let mut best_t = f32::INFINITY;
        let mut best: Option<(Vec3, Vec3)> = None;
        let positions = &mesh.positions;
        for tri in mesh.indices.chunks_exact(3) {
            let a = Vec3::from_array(positions[tri[0] as usize]);
            let b = Vec3::from_array(positions[tri[1] as usize]);
            let c = Vec3::from_array(positions[tri[2] as usize]);
            let Some(t) = ray_triangle(ray, a, b, c) else {
                continue;
            };
            if t < best_t {
                let mut normal = (b - a).cross(c - a);
                if normal.length_squared() < 1e-12 {
                    continue;
                }
                normal = normal.normalize();
                if normal.dot(ray.direction) > 0.0 {
                    normal = -normal;
                }
                best_t = t;
                best = Some(((a + b + c) / 3.0, normal));
            }
        }
        best
    }

    /// Outward normal of the triangle the ray hits first.
    pub fn face_normal(&self, id: ObjectId, ray: Ray) -> Option<Vec3> {
        let mesh = self.object(id)?.world_mesh();
        let mut best_t = f32::INFINITY;
        let mut best: Option<Vec3> = None;
        let positions = &mesh.positions;
        for tri in mesh.indices.chunks_exact(3) {
            let a = Vec3::from_array(positions[tri[0] as usize]);
            let b = Vec3::from_array(positions[tri[1] as usize]);
            let c = Vec3::from_array(positions[tri[2] as usize]);
            let Some(t) = ray_triangle(ray, a, b, c) else {
                continue;
            };
            if t < best_t {
                let mut normal = (b - a).cross(c - a);
                if normal.length_squared() < 1e-12 {
                    continue;
                }
                normal = normal.normalize();
                if normal.dot(ray.direction) > 0.0 {
                    normal = -normal;
                }
                best_t = t;
                best = Some(normal);
            }
        }
        best
    }

    fn world_bounds(&self, id: ObjectId) -> Option<(Vec3, Vec3)> {
        let object = self.object(id)?;
        let mesh = object.world_mesh();
        if mesh.positions.is_empty() {
            return None;
        }
        Some(mesh.bounds())
    }

    fn combined_bounds(&self, ids: &[ObjectId]) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        let mut any = false;
        for id in ids {
            if let Some((lo, hi)) = self.world_bounds(*id) {
                min = min.min(lo);
                max = max.max(hi);
                any = true;
            }
        }
        any.then_some((min, max))
    }
}

fn ray_triangle(ray: Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let edge1 = b - a;
    let edge2 = c - a;
    let p = ray.direction.cross(edge2);
    let det = edge1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = ray.origin - a;
    let u = tvec.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = tvec.cross(edge1);
    let v = ray.direction.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv;
    (t >= 0.0).then_some(t)
}

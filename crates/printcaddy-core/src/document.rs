use glam::Vec3;

use glam::Mat4;

use crate::boolean::{boolean, BooleanError, BooleanOp};
use crate::color::{Color, DEFAULT_MODEL_COLOR};
use crate::mesh::{box_mesh, cylinder_mesh, Mesh};
use crate::ray::{ray_aabb, ray_cylinder, Ray};
use crate::sketch::{build_sketch, seat_sketch, SketchDef, SketchError};
use crate::solids::{filleted_box, shape_mesh, FilletError, Shape};

pub const DEFAULT_BOX: (f32, f32, f32) = (20.0, 20.0, 20.0);
pub const CYLINDER_RADIUS: f32 = 10.0;
pub const CYLINDER_HEIGHT: f32 = 20.0;
pub const CYLINDER_SEGMENTS: u32 = 32;
/// The current app's default snap step, in millimetres.
pub const DEFAULT_SNAP_MM: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u64);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ObjectKind {
    Box {
        width: f32,
        depth: f32,
        height: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        segments: u32,
    },
    Primitive {
        shape: Shape,
        width: f32,
        depth: f32,
        height: f32,
    },
    Mesh {
        mesh: Mesh,
    },
    Sketch {
        def: SketchDef,
    },
    FilletedBox {
        width: f32,
        depth: f32,
        height: f32,
        radius: f32,
    },
    Group {
        members: Vec<ObjectId>,
        cut: Option<Mesh>,
        width: f32,
        depth: f32,
        height: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupError {
    TooFew,
    OnlyHoles,
    Boolean(BooleanError),
}

impl std::fmt::Display for GroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupError::TooFew => f.write_str("Select at least two bodies to group."),
            GroupError::OnlyHoles => f.write_str("A hole needs a solid in the same group."),
            GroupError::Boolean(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub kind: ObjectKind,
    pub color: Color,
    /// Bottom-centre of the unrotated body, in millimetres. `y` is how far the bottom sits above the plate.
    pub position: Vec3,
    /// Euler angles in degrees, applied in X then Y then Z around the body's centre.
    pub rotation: Vec3,
    /// Bumps when the mesh itself changes, so the viewport uploads it again.
    pub revision: u64,
    /// A cut-out. It is drawn see-through, and grouping it with a solid cuts the solid.
    pub hole: bool,
    /// Set when this body belongs to a group. A click selects the group instead.
    pub parent: Option<ObjectId>,
}

impl Object {
    pub fn local_mesh(&self) -> Mesh {
        match &self.kind {
            ObjectKind::Box {
                width,
                depth,
                height,
            } => box_mesh(*width, *depth, *height),
            ObjectKind::Cylinder {
                radius,
                height,
                segments,
            } => cylinder_mesh(*radius, *height, *segments),
            ObjectKind::Primitive {
                shape,
                width,
                depth,
                height,
            } => shape_mesh(*shape, *width, *depth, *height),
            ObjectKind::Mesh { mesh } => mesh.clone(),
            ObjectKind::Sketch { def } => build_sketch(def).unwrap_or_else(|_| Mesh {
                positions: Vec::new(),
                normals: Vec::new(),
                indices: Vec::new(),
            }),
            ObjectKind::FilletedBox {
                width,
                depth,
                height,
                radius,
            } => filleted_box(*width, *depth, *height, *radius)
                .unwrap_or_else(|_| box_mesh(*width, *depth, *height)),
            ObjectKind::Group { cut, .. } => cut.clone().unwrap_or_else(empty_mesh),
        }
    }

    pub fn size(&self) -> (f32, f32, f32) {
        match &self.kind {
            ObjectKind::Box {
                width,
                depth,
                height,
            } => (*width, *depth, *height),
            ObjectKind::Cylinder { radius, height, .. } => (radius * 2.0, radius * 2.0, *height),
            ObjectKind::Primitive {
                width,
                depth,
                height,
                ..
            } => (*width, *depth, *height),
            ObjectKind::Mesh { mesh } => {
                let (min, max) = mesh.bounds();
                (max.x - min.x, max.z - min.z, max.y - min.y)
            }
            ObjectKind::Sketch { .. } => {
                let (min, max) = self.local_mesh().bounds();
                (max.x - min.x, max.z - min.z, max.y - min.y)
            }
            ObjectKind::FilletedBox {
                width,
                depth,
                height,
                ..
            } => (*width, *depth, *height),
            ObjectKind::Group {
                width,
                depth,
                height,
                ..
            } => (*width, *depth, *height),
        }
    }

    pub fn shape(&self) -> Option<Shape> {
        match &self.kind {
            ObjectKind::Box { .. } => Some(Shape::Box),
            ObjectKind::Cylinder { .. } => Some(Shape::Cylinder),
            ObjectKind::Primitive { shape, .. } => Some(*shape),
            ObjectKind::FilletedBox { .. } => Some(Shape::RoundedBox),
            ObjectKind::Mesh { .. } | ObjectKind::Sketch { .. } | ObjectKind::Group { .. } => None,
        }
    }

    /// Local point to world: turn around the centre, then place the bottom-centre at `position`.
    pub fn model_matrix(&self) -> Mat4 {
        let (_, _, height) = self.size();
        let center = Vec3::new(0.0, height * 0.5, 0.0);
        let rot = Mat4::from_euler(
            glam::EulerRot::XYZ,
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        );
        Mat4::from_translation(self.position + center) * rot * Mat4::from_translation(-center)
    }

    pub fn world_mesh(&self) -> Mesh {
        let model = self.model_matrix();
        let normal_matrix = model.inverse().transpose();
        let mesh = self.local_mesh();
        Mesh {
            positions: mesh
                .positions
                .iter()
                .map(|p| model.transform_point3(Vec3::from_array(*p)).to_array())
                .collect(),
            normals: mesh
                .normals
                .iter()
                .map(|n| {
                    normal_matrix
                        .transform_vector3(Vec3::from_array(*n))
                        .normalize_or_zero()
                        .to_array()
                })
                .collect(),
            indices: mesh.indices,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    objects: Vec<Object>,
    next_id: u64,
    box_count: u32,
    cylinder_count: u32,
    selection: Vec<ObjectId>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
            box_count: 0,
            cylinder_count: 0,
            selection: Vec::new(),
        }
    }

    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    pub(crate) fn next_id_value(&self) -> u64 {
        self.next_id
    }

    pub(crate) fn from_objects(objects: Vec<Object>, next_id: u64) -> Self {
        let mut doc = Self::new();
        doc.box_count = objects
            .iter()
            .filter(|object| matches!(object.kind, ObjectKind::Box { .. }))
            .count() as u32;
        doc.cylinder_count = objects
            .iter()
            .filter(|object| matches!(object.kind, ObjectKind::Cylinder { .. }))
            .count() as u32;
        let highest = objects.iter().map(|object| object.id.0).max().unwrap_or(0);
        doc.next_id = next_id.max(highest.saturating_add(1)).max(1);
        doc.objects = objects;
        doc
    }

    pub fn set_color(&mut self, id: ObjectId, color: crate::color::Color) {
        if let Some(object) = self.object_mut(id) {
            object.color = color;
            object.revision = object.revision.saturating_add(1);
        }
    }

    pub fn sketch(&self, id: ObjectId) -> Option<&SketchDef> {
        match &self.object(id)?.kind {
            ObjectKind::Sketch { def } => Some(def),
            _ => None,
        }
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    /// The last selected body. Properties and the status line follow this one.
    pub fn selection(&self) -> Option<ObjectId> {
        self.selection.last().copied()
    }

    pub fn selected_ids(&self) -> &[ObjectId] {
        &self.selection
    }

    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selection.contains(&id)
    }

    pub fn select(&mut self, id: Option<ObjectId>) {
        self.selection.clear();
        if let Some(id) = id.filter(|id| self.objects.iter().any(|object| object.id == *id)) {
            self.selection.push(id);
        }
    }

    /// Shift-click and the touch “add” switch. A selected body is removed; a new one is added.
    pub fn toggle_select(&mut self, id: ObjectId) {
        if !self.objects.iter().any(|object| object.id == id) {
            return;
        }
        if let Some(index) = self.selection.iter().position(|selected| *selected == id) {
            self.selection.remove(index);
        } else {
            self.selection.push(id);
        }
    }

    pub fn add_box(&mut self) -> ObjectId {
        self.box_count += 1;
        let (width, depth, height) = DEFAULT_BOX;
        let name = format!("Box {}", self.box_count);
        self.spawn(
            name,
            ObjectKind::Box {
                width,
                depth,
                height,
            },
        )
    }

    pub fn add_cylinder(&mut self) -> ObjectId {
        self.cylinder_count += 1;
        let name = format!("Cylinder {}", self.cylinder_count);
        self.spawn(
            name,
            ObjectKind::Cylinder {
                radius: CYLINDER_RADIUS,
                height: CYLINDER_HEIGHT,
                segments: CYLINDER_SEGMENTS,
            },
        )
    }

    /// Adds an imported mesh. The mesh is recentered in XZ and dropped onto the plate.
    pub fn add_shape(&mut self, shape: Shape) -> ObjectId {
        match shape {
            Shape::Box => self.add_box(),
            Shape::Cylinder => self.add_cylinder(),
            other => {
                let (width, depth, height) = other.default_size();
                let name = self.next_shape_name(other);
                self.spawn(
                    name,
                    ObjectKind::Primitive {
                        shape: other,
                        width,
                        depth,
                        height,
                    },
                )
            }
        }
    }

    pub fn add_sketch(&mut self, def: SketchDef) -> Result<ObjectId, SketchError> {
        let (seated, at) = seat_sketch(def)?;
        let count = self
            .objects
            .iter()
            .filter(|object| matches!(object.kind, ObjectKind::Sketch { .. }))
            .count()
            + 1;
        let id = self.spawn(
            format!("Sketch {count}"),
            ObjectKind::Sketch { def: seated },
        );
        if let Some(object) = self.object_mut(id) {
            // The footprint stays centred, and the lowest point sits on the plate.
            object.position = Vec3::new(at.x, 0.0, at.z);
        }
        Ok(id)
    }

    pub fn replace_sketch(&mut self, id: ObjectId, def: SketchDef) -> Result<(), SketchError> {
        let (seated, at) = seat_sketch(def)?;
        let object = self
            .object_mut(id)
            .ok_or(SketchError::TooFewProfilePoints)?;
        object.kind = ObjectKind::Sketch { def: seated };
        // `at` is the new bottom-centre in the sketch's own coordinates, which are
        // already local for an edited body. Adding it keeps the body's place on the plate.
        object.position += at;
        object.rotation = Vec3::ZERO;
        object.revision = object.revision.saturating_add(1);
        Ok(())
    }

    pub fn add_mesh(&mut self, name: impl Into<String>, mut mesh: Mesh) -> ObjectId {
        seat_on_plate(&mut mesh);
        self.spawn(name.into(), ObjectKind::Mesh { mesh })
    }

    pub fn move_object_to(&mut self, id: ObjectId, x: f32, z: f32) {
        if let Some(object) = self.object_mut(id) {
            object.position.x = x;
            object.position.z = z;
        }
    }

    pub fn set_position(&mut self, id: ObjectId, position: Vec3) {
        let Some(current) = self.object(id).map(|object| object.position) else {
            return;
        };
        self.shift_tree(id, position - current);
    }

    pub fn set_rotation(&mut self, id: ObjectId, degrees: Vec3) {
        if let Some(object) = self.object_mut(id) {
            object.rotation = degrees;
        }
    }

    pub fn set_size(&mut self, id: ObjectId, width: f32, depth: f32, height: f32) {
        let width = width.max(0.2);
        let depth = depth.max(0.2);
        let height = height.max(0.2);
        let (cw, cd, ch) = match self.object(id) {
            Some(object) => object.size(),
            None => return,
        };
        let Some(object) = self.object_mut(id) else {
            return;
        };
        match &mut object.kind {
            ObjectKind::Box {
                width: w,
                depth: d,
                height: h,
            } => {
                *w = width;
                *d = depth;
                *h = height;
            }
            ObjectKind::Cylinder {
                radius, height: h, ..
            } => {
                let diameter = if (width - *radius * 2.0).abs() >= (depth - *radius * 2.0).abs() {
                    width
                } else {
                    depth
                };
                *radius = diameter * 0.5;
                *h = height;
            }
            ObjectKind::Primitive {
                width: w,
                depth: d,
                height: h,
                ..
            } => {
                *w = width;
                *d = depth;
                *h = height;
            }
            ObjectKind::Mesh { mesh } => scale_mesh_to(mesh, width, depth, height),
            ObjectKind::Sketch { def } => {
                let sx = width / cw.max(0.2);
                let sz = depth / cd.max(0.2);
                let sy = height / ch.max(0.2);
                for point in &mut def.profile {
                    point[0] *= sx;
                    point[1] *= sz;
                }
                for point in &mut def.path {
                    point[0] *= sx;
                    point[1] *= sz;
                }
                def.height = (def.height * sy).max(0.2);
            }
            ObjectKind::FilletedBox {
                width: w,
                depth: d,
                height: h,
                radius,
            } => {
                *w = width;
                *d = depth;
                *h = height;
                let limit = width.min(depth).min(height) * 0.5 - 0.2;
                *radius = radius.min(limit.max(0.2));
            }
            ObjectKind::Group {
                cut,
                width: w,
                depth: d,
                height: h,
                ..
            } => {
                *w = width;
                *d = depth;
                *h = height;
                if let Some(mesh) = cut {
                    scale_mesh_to(mesh, width, depth, height);
                }
            }
        }
        object.revision = object.revision.saturating_add(1);
    }

    pub fn delete_selected(&mut self) {
        let mut doomed = self.selection.clone();
        for id in self.selection.clone() {
            doomed.extend(self.descendants(id));
        }
        self.objects.retain(|object| !doomed.contains(&object.id));
        self.selection.clear();
        self.detach_missing();
    }

    pub fn delete(&mut self, id: ObjectId) {
        self.objects.retain(|object| object.id != id);
        self.selection.retain(|selected| *selected != id);
    }

    /// Move every selected body by the same delta. Y is included so the lift arrow can raise them.
    pub fn translate_selected(&mut self, delta: Vec3) {
        let ids = self.selection.clone();
        for id in ids {
            self.shift_tree(id, delta);
        }
    }

    /// World-axis box around the selection. None when nothing is selected.
    pub fn selection_bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for id in &self.selection {
            let (lo, hi) = self.tree_bounds(&[*id]);
            min = min.min(lo);
            max = max.max(hi);
        }
        if min.x.is_finite() && max.x > min.x - 1.0 {
            Some((min, max))
        } else {
            None
        }
    }

    /// Write every non-zero angle into the vertices and store 0, 0, 0.
    /// The body becomes a mesh seated on its new world bottom-centre, so the
    /// handles and dimension lines sit on the plate axes again.
    pub fn bake_rotations(&mut self) {
        let ids: Vec<ObjectId> = self
            .objects
            .iter()
            .filter(|object| object.rotation.length_squared() > 1e-8)
            .map(|object| object.id)
            .collect();
        for id in ids {
            self.bake_one(id);
        }
    }

    fn bake_one(&mut self, id: ObjectId) {
        let Some(object) = self.object(id) else {
            return;
        };
        if object.rotation.length_squared() <= 1e-8 {
            return;
        }
        if let ObjectKind::Sketch { def } = &object.kind {
            let model = object.model_matrix();
            let mut turned = def.clone();
            turned.origin = model.transform_point3(Vec3::from(def.origin)).to_array();
            turned.axis_u = model
                .transform_vector3(Vec3::from(def.axis_u))
                .normalize_or_zero()
                .to_array();
            turned.axis_v = model
                .transform_vector3(Vec3::from(def.axis_v))
                .normalize_or_zero()
                .to_array();
            if let Ok((seated, at)) = seat_sketch(turned) {
                let object = self.object_mut(id).unwrap();
                object.kind = ObjectKind::Sketch { def: seated };
                object.position = at;
                object.rotation = Vec3::ZERO;
                object.revision = object.revision.saturating_add(1);
            }
            return;
        }
        if matches!(object.kind, ObjectKind::Group { .. }) {
            self.bake_group(id);
            return;
        }
        let mut mesh = object.world_mesh();
        let (min, max) = mesh.bounds();
        let center_x = (min.x + max.x) * 0.5;
        let center_z = (min.z + max.z) * 0.5;
        for point in &mut mesh.positions {
            point[0] -= center_x;
            point[1] -= min.y;
            point[2] -= center_z;
        }
        let object = self.object_mut(id).unwrap();
        object.kind = ObjectKind::Mesh { mesh };
        object.position = Vec3::new(center_x, min.y, center_z);
        object.rotation = Vec3::ZERO;
        object.revision = object.revision.saturating_add(1);
    }

    pub fn duplicate(&mut self, id: ObjectId) -> Option<ObjectId> {
        let source = self.object(id)?;
        let (width, _, _) = source.size();
        let copy = self.clone_tree(id, Vec3::new(width + 2.0, 0.0, 0.0))?;
        self.select(Some(copy));
        Some(copy)
    }

    /// Closest body the ray enters, if any. A member of a group selects the group.
    pub fn hit_test(&self, ray: Ray) -> Option<ObjectId> {
        let mut best: Option<(f32, ObjectId)> = None;
        for object in &self.objects {
            if object.parent.is_some() {
                continue;
            }
            let hit = if let ObjectKind::Group { cut: None, .. } = &object.kind {
                self.descendants(object.id)
                    .iter()
                    .filter_map(|id| {
                        let member = self.object(*id)?;
                        hit_transformed(member, self.display_matrix(*id), ray)
                    })
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            } else {
                hit_object(object, ray)
            };
            if let Some(t) = hit {
                if best.map(|(bt, _)| t < bt).unwrap_or(true) {
                    best = Some((t, object.id));
                }
            }
        }
        best.map(|(_, id)| id)
    }

    pub fn display_matrix(&self, id: ObjectId) -> Mat4 {
        let Some(object) = self.object(id) else {
            return Mat4::IDENTITY;
        };
        let mut matrix = object.model_matrix();
        let mut parent = object.parent;
        while let Some(parent_id) = parent {
            let Some(group) = self.object(parent_id) else {
                break;
            };
            if group.rotation.length_squared() > 1e-8 {
                let (_, _, height) = group.size();
                let center = group.position + Vec3::new(0.0, height * 0.5, 0.0);
                let spin = Mat4::from_translation(center)
                    * Mat4::from_euler(
                        glam::EulerRot::XYZ,
                        group.rotation.x.to_radians(),
                        group.rotation.y.to_radians(),
                        group.rotation.z.to_radians(),
                    )
                    * Mat4::from_translation(-center);
                matrix = spin * matrix;
            }
            parent = group.parent;
        }
        matrix
    }

    pub fn display_alpha(&self, id: ObjectId) -> f32 {
        if self.object(id).is_some_and(|object| object.hole) {
            0.38
        } else {
            1.0
        }
    }

    pub fn is_shown(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if self.hidden_by_cut(id) {
            return false;
        }
        if let ObjectKind::Group { cut: None, .. } = &object.kind {
            return false;
        }
        true
    }

    pub fn boolean_selected(&mut self, op: BooleanOp) -> Result<(), BooleanError> {
        let ids: Vec<ObjectId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|object| object.parent.is_none())
            })
            .collect();
        if ids.len() < 2 {
            return Err(BooleanError::TooFew);
        }
        let meshes = ids
            .iter()
            .map(|id| self.solid_mesh(*id))
            .collect::<Result<Vec<_>, _>>()?;
        let combined = match op {
            BooleanOp::Union => fold_boolean(BooleanOp::Union, &meshes)?,
            BooleanOp::Intersect => fold_boolean(BooleanOp::Intersect, &meshes)?,
            BooleanOp::Subtract => {
                let stock = meshes.last().cloned().ok_or(BooleanError::TooFew)?;
                let mut acc = stock;
                for tool in &meshes[..meshes.len() - 1] {
                    acc = boolean(BooleanOp::Subtract, &acc, tool)?;
                }
                acc
            }
        };
        let name = match op {
            BooleanOp::Union => "Join",
            BooleanOp::Subtract => "Subtract",
            BooleanOp::Intersect => "Intersect",
        };
        self.delete_ids(&ids);
        self.add_seated_mesh(name, combined);
        Ok(())
    }

    pub fn group_selected(&mut self) -> Result<ObjectId, GroupError> {
        let ids: Vec<ObjectId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|object| object.parent.is_none())
            })
            .collect();
        if ids.len() < 2 {
            return Err(GroupError::TooFew);
        }
        let any_hole = ids
            .iter()
            .any(|id| self.object(*id).is_some_and(|object| object.hole));
        let (cut, position, size) = if any_hole {
            let mesh = match self.cut_members(&ids) {
                Ok(mesh) => mesh,
                Err(BooleanError::Empty) => return Err(GroupError::OnlyHoles),
                Err(err) => return Err(GroupError::Boolean(err)),
            };
            let (min, max) = mesh.bounds();
            let position = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
            let mut local = mesh;
            seat_on_plate(&mut local);
            (
                Some(local),
                position,
                (max.x - min.x, max.z - min.z, max.y - min.y),
            )
        } else {
            let (min, max) = self.tree_bounds(&ids);
            (
                None,
                Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5),
                (max.x - min.x, max.z - min.z, max.y - min.y),
            )
        };
        let count = self
            .objects
            .iter()
            .filter(|object| matches!(object.kind, ObjectKind::Group { .. }))
            .count()
            + 1;
        let id = self.allocate(
            format!("Group {count}"),
            ObjectKind::Group {
                members: ids.clone(),
                cut,
                width: size.0.max(0.2),
                depth: size.1.max(0.2),
                height: size.2.max(0.2),
            },
            position,
        );
        for member in ids {
            if let Some(object) = self.object_mut(member) {
                object.parent = Some(id);
            }
        }
        self.select(Some(id));
        Ok(id)
    }

    pub fn ungroup_selected(&mut self) -> Result<(), GroupError> {
        let ids = self.selection.clone();
        let mut released = Vec::new();
        for id in ids {
            let Some(object) = self.object(id) else {
                continue;
            };
            let ObjectKind::Group { members, .. } = &object.kind else {
                continue;
            };
            let members = members.clone();
            let parent = object.parent;
            for member in &members {
                if let Some(child) = self.object_mut(*member) {
                    child.parent = parent;
                }
            }
            released.extend(members);
            self.objects.retain(|object| object.id != id);
        }
        if released.is_empty() {
            return Err(GroupError::TooFew);
        }
        self.selection = released;
        self.detach_missing();
        Ok(())
    }

    pub fn set_hole(&mut self, id: ObjectId, hole: bool) {
        if let Some(object) = self.object_mut(id) {
            object.hole = hole;
            object.revision = object.revision.saturating_add(1);
        }
    }

    pub fn fillet(&mut self, id: ObjectId, radius: f32) -> Result<(), FilletError> {
        let Some(object) = self.object(id) else {
            return Err(FilletError::NotABox);
        };
        let (width, depth, height) = match &object.kind {
            ObjectKind::Box {
                width,
                depth,
                height,
            }
            | ObjectKind::FilletedBox {
                width,
                depth,
                height,
                ..
            } => (*width, *depth, *height),
            _ => return Err(FilletError::NotABox),
        };
        let _ = filleted_box(width, depth, height, radius)?;
        let Some(object) = self.object_mut(id) else {
            return Err(FilletError::NotABox);
        };
        object.kind = ObjectKind::FilletedBox {
            width,
            depth,
            height,
            radius,
        };
        object.revision = object.revision.saturating_add(1);
        Ok(())
    }

    pub fn motion_ids(&self) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        for id in &self.selection {
            ids.push(*id);
            ids.extend(self.descendants(*id));
        }
        ids
    }

    fn descendants(&self, id: ObjectId) -> Vec<ObjectId> {
        let mut out = Vec::new();
        let Some(object) = self.object(id) else {
            return out;
        };
        if let ObjectKind::Group { members, .. } = &object.kind {
            for member in members.clone() {
                out.push(member);
                out.extend(self.descendants(member));
            }
        }
        out
    }

    fn shift_tree(&mut self, id: ObjectId, delta: Vec3) {
        if delta.length_squared() == 0.0 {
            return;
        }
        let children = self.descendants(id);
        if let Some(object) = self.object_mut(id) {
            object.position += delta;
        }
        for child in children {
            if let Some(object) = self.object_mut(child) {
                object.position += delta;
            }
        }
    }

    fn hidden_by_cut(&self, id: ObjectId) -> bool {
        let mut parent = self.object(id).and_then(|object| object.parent);
        while let Some(parent_id) = parent {
            let Some(group) = self.object(parent_id) else {
                return false;
            };
            if matches!(group.kind, ObjectKind::Group { cut: Some(_), .. }) {
                return true;
            }
            parent = group.parent;
        }
        false
    }

    fn solid_mesh(&self, id: ObjectId) -> Result<Mesh, BooleanError> {
        let Some(object) = self.object(id) else {
            return Err(BooleanError::NotClosed);
        };
        if let ObjectKind::Group { members, cut, .. } = &object.kind {
            if cut.is_some() {
                return Ok(self.transformed_mesh(id));
            }
            let meshes = members
                .iter()
                .map(|member| self.solid_mesh(*member))
                .collect::<Result<Vec<_>, _>>()?;
            return fold_boolean(BooleanOp::Union, &meshes);
        }
        let mesh = self.transformed_mesh(id);
        if mesh.triangle_count() == 0 {
            return Err(BooleanError::NotClosed);
        }
        Ok(mesh)
    }

    fn transformed_mesh(&self, id: ObjectId) -> Mesh {
        let Some(object) = self.object(id) else {
            return empty_mesh();
        };
        let model = self.display_matrix(id);
        let normal_matrix = model.inverse().transpose();
        let mesh = object.local_mesh();
        Mesh {
            positions: mesh
                .positions
                .iter()
                .map(|p| model.transform_point3(Vec3::from_array(*p)).to_array())
                .collect(),
            normals: mesh
                .normals
                .iter()
                .map(|n| {
                    normal_matrix
                        .transform_vector3(Vec3::from_array(*n))
                        .normalize_or_zero()
                        .to_array()
                })
                .collect(),
            indices: mesh.indices,
        }
    }

    fn cut_members(&self, ids: &[ObjectId]) -> Result<Mesh, BooleanError> {
        let mut solids = Vec::new();
        let mut holes = Vec::new();
        for id in ids {
            let mesh = self.solid_mesh(*id)?;
            if self.object(*id).is_some_and(|object| object.hole) {
                holes.push(mesh);
            } else {
                solids.push(mesh);
            }
        }
        if solids.is_empty() {
            return Err(BooleanError::Empty);
        }
        let mut acc = fold_boolean(BooleanOp::Union, &solids)?;
        if !holes.is_empty() {
            let tool = fold_boolean(BooleanOp::Union, &holes)?;
            acc = boolean(BooleanOp::Subtract, &acc, &tool)?;
        }
        Ok(acc)
    }

    fn tree_bounds(&self, ids: &[ObjectId]) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for id in ids {
            if matches!(
                self.object(*id).map(|object| &object.kind),
                Some(ObjectKind::Group { cut: Some(_), .. })
            ) {
                let (lo, hi) = self.transformed_mesh(*id).bounds();
                min = min.min(lo);
                max = max.max(hi);
                continue;
            }
            for leaf in std::iter::once(*id).chain(self.descendants(*id)) {
                if matches!(
                    self.object(leaf).map(|o| &o.kind),
                    Some(ObjectKind::Group { .. })
                ) {
                    continue;
                }
                let (lo, hi) = self.transformed_mesh(leaf).bounds();
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.x.is_finite() {
            (min, max)
        } else {
            (Vec3::ZERO, Vec3::ZERO)
        }
    }

    fn delete_ids(&mut self, ids: &[ObjectId]) {
        let mut doomed = ids.to_vec();
        for id in ids {
            doomed.extend(self.descendants(*id));
        }
        self.objects.retain(|object| !doomed.contains(&object.id));
        self.selection.clear();
        self.detach_missing();
    }

    fn detach_missing(&mut self) {
        let live: Vec<ObjectId> = self.objects.iter().map(|object| object.id).collect();
        for object in &mut self.objects {
            if object.parent.is_some_and(|parent| !live.contains(&parent)) {
                object.parent = None;
            }
            if let ObjectKind::Group { members, .. } = &mut object.kind {
                members.retain(|member| live.contains(member));
            }
        }
    }

    fn add_seated_mesh(&mut self, name: &str, mut mesh: Mesh) -> ObjectId {
        let (min, max) = mesh.bounds();
        let at = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
        seat_on_plate(&mut mesh);
        let id = self.allocate(name.to_string(), ObjectKind::Mesh { mesh }, at);
        self.select(Some(id));
        id
    }

    fn clone_tree(&mut self, id: ObjectId, delta: Vec3) -> Option<ObjectId> {
        let source = self.object(id)?.clone();
        let new_id = self.allocate(
            format!("{} copy", source.name),
            source.kind.clone(),
            source.position + delta,
        );
        if let Some(object) = self.object_mut(new_id) {
            object.rotation = source.rotation;
            object.color = source.color;
            object.hole = source.hole;
        }
        if let ObjectKind::Group { members, .. } = source.kind {
            let mut copied = Vec::new();
            for member in members {
                if let Some(child) = self.clone_tree(member, delta) {
                    if let Some(object) = self.object_mut(child) {
                        object.parent = Some(new_id);
                    }
                    copied.push(child);
                }
            }
            if let Some(object) = self.object_mut(new_id) {
                if let ObjectKind::Group { members, .. } = &mut object.kind {
                    *members = copied;
                }
            }
        }
        Some(new_id)
    }

    fn allocate(&mut self, name: String, kind: ObjectKind, position: Vec3) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push(Object {
            id,
            name,
            kind,
            color: DEFAULT_MODEL_COLOR,
            position,
            rotation: Vec3::ZERO,
            revision: 1,
            hole: false,
            parent: None,
        });
        id
    }

    fn bake_group(&mut self, id: ObjectId) {
        let Some(object) = self.object(id) else {
            return;
        };
        let rotation = object.rotation;
        if rotation.length_squared() <= 1e-8 {
            return;
        }
        let (_, _, height) = object.size();
        let center = object.position + Vec3::new(0.0, height * 0.5, 0.0);
        let turn = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            rotation.x.to_radians(),
            rotation.y.to_radians(),
            rotation.z.to_radians(),
        );
        let members = self.descendants(id);
        for member in &members {
            let Some(object) = self.object(*member) else {
                continue;
            };
            let (_, _, member_height) = object.size();
            let member_center = object.position + Vec3::new(0.0, member_height * 0.5, 0.0);
            let new_center = center + turn * (member_center - center);
            let start_q = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                object.rotation.x.to_radians(),
                object.rotation.y.to_radians(),
                object.rotation.z.to_radians(),
            );
            let (rx, ry, rz) = (turn * start_q).to_euler(glam::EulerRot::XYZ);
            if let Some(object) = self.object_mut(*member) {
                object.position = new_center - Vec3::new(0.0, member_height * 0.5, 0.0);
                object.rotation = Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
            }
        }
        if let Some(object) = self.object_mut(id) {
            object.rotation = Vec3::ZERO;
        }
        if let Some(object) = self.object(id) {
            if matches!(object.kind, ObjectKind::Group { cut: Some(_), .. }) {
                let mut mesh = object.world_mesh();
                let (min, max) = mesh.bounds();
                let at = Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
                seat_on_plate(&mut mesh);
                if let Some(object) = self.object_mut(id) {
                    if let ObjectKind::Group {
                        cut,
                        width,
                        depth,
                        height,
                        ..
                    } = &mut object.kind
                    {
                        *width = max.x - min.x;
                        *depth = max.z - min.z;
                        *height = max.y - min.y;
                        *cut = Some(mesh);
                    }
                    object.position = at;
                    object.revision = object.revision.saturating_add(1);
                }
            }
        }
        for member in members {
            self.bake_one(member);
        }
    }

    fn spawn(&mut self, name: String, kind: ObjectKind) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        let index = self.objects.len() as f32;
        // Step each new body along X so it does not sit inside the previous one.
        let position = Vec3::new(index * 40.0, 0.0, 0.0);
        self.objects.push(Object {
            id,
            name,
            kind,
            color: DEFAULT_MODEL_COLOR,
            position,
            rotation: Vec3::ZERO,
            revision: 1,
            hole: false,
            parent: None,
        });
        self.selection.clear();
        self.selection.push(id);
        id
    }

    fn next_shape_name(&self, shape: Shape) -> String {
        let count = self
            .objects
            .iter()
            .filter(|object| object.shape() == Some(shape))
            .count()
            + 1;
        format!("{} {}", shape.label(), count)
    }
}

fn seat_on_plate(mesh: &mut Mesh) {
    if mesh.positions.is_empty() {
        return;
    }
    let (min, max) = mesh.bounds();
    let cx = (min.x + max.x) * 0.5;
    let cz = (min.z + max.z) * 0.5;
    for p in &mut mesh.positions {
        p[0] -= cx;
        p[1] -= min.y;
        p[2] -= cz;
    }
}

fn scale_mesh_to(mesh: &mut Mesh, width: f32, depth: f32, height: f32) {
    let (min, max) = mesh.bounds();
    let size = max - min;
    let sx = if size.x.abs() < 1e-6 {
        1.0
    } else {
        width / size.x
    };
    let sy = if size.y.abs() < 1e-6 {
        1.0
    } else {
        height / size.y
    };
    let sz = if size.z.abs() < 1e-6 {
        1.0
    } else {
        depth / size.z
    };
    for p in &mut mesh.positions {
        p[0] *= sx;
        p[1] *= sy;
        p[2] *= sz;
    }
}

fn hit_object(object: &Object, ray: Ray) -> Option<f32> {
    let inverse = object.model_matrix().inverse();
    let origin = inverse.transform_point3(ray.origin);
    let end = inverse.transform_point3(ray.origin + ray.direction);
    let local = Ray::new(origin, end - origin);
    match &object.kind {
        ObjectKind::Box {
            width,
            depth,
            height,
        }
        | ObjectKind::Primitive {
            width,
            depth,
            height,
            ..
        } => {
            let min = Vec3::new(-width * 0.5, 0.0, -depth * 0.5);
            let max = Vec3::new(width * 0.5, *height, depth * 0.5);
            ray_aabb(local, min, max)
        }
        ObjectKind::Cylinder { radius, height, .. } => ray_cylinder(local, *radius, *height),
        ObjectKind::Mesh { mesh } => {
            let (min, max) = mesh.bounds();
            ray_aabb(local, min, max)
        }
        ObjectKind::Sketch { .. } | ObjectKind::FilletedBox { .. } => {
            let (min, max) = object.local_mesh().bounds();
            ray_aabb(local, min, max)
        }
        ObjectKind::Group { cut, .. } => {
            let mesh = cut.clone().unwrap_or_else(empty_mesh);
            let (min, max) = mesh.bounds();
            ray_aabb(local, min, max)
        }
    }
}

fn fold_boolean(op: BooleanOp, meshes: &[Mesh]) -> Result<Mesh, BooleanError> {
    let mut acc = meshes.first().cloned().ok_or(BooleanError::TooFew)?;
    for mesh in meshes.iter().skip(1) {
        acc = boolean(op, &acc, mesh)?;
    }
    Ok(acc)
}

fn hit_transformed(object: &Object, model: Mat4, ray: Ray) -> Option<f32> {
    let inverse = model.inverse();
    let origin = inverse.transform_point3(ray.origin);
    let end = inverse.transform_point3(ray.origin + ray.direction);
    let local = Ray::new(origin, end - origin);
    let mesh = object.local_mesh();
    if mesh.positions.is_empty() {
        return None;
    }
    let (min, max) = mesh.bounds();
    ray_aabb(local, min, max)
}

fn empty_mesh() -> Mesh {
    Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    }
}

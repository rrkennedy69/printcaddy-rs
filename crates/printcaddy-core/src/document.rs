use glam::Vec3;

use glam::Mat4;

use crate::color::{Color, DEFAULT_MODEL_COLOR};
use crate::mesh::{box_mesh, cylinder_mesh, Mesh};
use crate::ray::{ray_aabb, ray_cylinder, Ray};
use crate::solids::{shape_mesh, Shape};

pub const DEFAULT_BOX: (f32, f32, f32) = (20.0, 20.0, 20.0);
pub const CYLINDER_RADIUS: f32 = 10.0;
pub const CYLINDER_HEIGHT: f32 = 20.0;
pub const CYLINDER_SEGMENTS: u32 = 32;
/// The current app's default snap step, in millimetres.
pub const DEFAULT_SNAP_MM: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(pub u64);

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug)]
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
        }
    }

    pub fn shape(&self) -> Option<Shape> {
        match &self.kind {
            ObjectKind::Box { .. } => Some(Shape::Box),
            ObjectKind::Cylinder { .. } => Some(Shape::Cylinder),
            ObjectKind::Primitive { shape, .. } => Some(*shape),
            ObjectKind::Mesh { .. } => None,
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
    selection: Option<ObjectId>,
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
            selection: None,
        }
    }

    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub fn selection(&self) -> Option<ObjectId> {
        self.selection
    }

    pub fn select(&mut self, id: Option<ObjectId>) {
        self.selection = id.filter(|id| self.objects.iter().any(|o| o.id == *id));
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
        if let Some(object) = self.object_mut(id) {
            object.position = position;
        }
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
        }
        object.revision = object.revision.saturating_add(1);
    }

    pub fn delete(&mut self, id: ObjectId) {
        self.objects.retain(|object| object.id != id);
        if self.selection == Some(id) {
            self.selection = None;
        }
    }

    pub fn duplicate(&mut self, id: ObjectId) -> Option<ObjectId> {
        let source = self.object(id)?.clone();
        let (width, _, _) = source.size();
        let mut copy = source;
        copy.position.x += width + 2.0;
        let name = format!("{} copy", copy.name);
        let kind = copy.kind;
        let id = self.spawn(name, kind);
        if let Some(object) = self.object_mut(id) {
            object.position = copy.position;
            object.rotation = copy.rotation;
            object.color = copy.color;
        }
        Some(id)
    }

    /// Closest body the ray enters, if any.
    pub fn hit_test(&self, ray: Ray) -> Option<ObjectId> {
        let mut best: Option<(f32, ObjectId)> = None;
        for object in &self.objects {
            if let Some(t) = hit_object(object, ray) {
                if best.map(|(bt, _)| t < bt).unwrap_or(true) {
                    best = Some((t, object.id));
                }
            }
        }
        best.map(|(_, id)| id)
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
        });
        self.selection = Some(id);
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
    }
}

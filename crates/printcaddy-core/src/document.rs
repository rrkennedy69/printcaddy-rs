use glam::Vec3;

use crate::color::{Color, DEFAULT_MODEL_COLOR};
use crate::mesh::{box_mesh, cylinder_mesh, Mesh};
use crate::ray::{ray_aabb, ray_cylinder, Ray};

pub const DEFAULT_BOX: (f32, f32, f32) = (20.0, 20.0, 20.0);
pub const CYLINDER_RADIUS: f32 = 10.0;
pub const CYLINDER_HEIGHT: f32 = 20.0;
pub const CYLINDER_SEGMENTS: u32 = 32;

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
    /// Bottom-centre of the body, in millimetres. `y` is how far the bottom sits above the plate.
    pub position: Vec3,
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
            ObjectKind::Mesh { mesh } => mesh.clone(),
        }
    }

    pub fn world_mesh(&self) -> Mesh {
        self.local_mesh().translated(self.position)
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
        });
        self.selection = Some(id);
        id
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

fn hit_object(object: &Object, ray: Ray) -> Option<f32> {
    let local = Ray::new(ray.origin - object.position, ray.direction);
    match &object.kind {
        ObjectKind::Box {
            width,
            depth,
            height,
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

//! PrintCaddy document model. This crate has no graphics or browser dependencies.

mod color;
mod document;
mod mesh;
mod ray;
mod stl;

pub use color::{Color, DEFAULT_MODEL_COLOR};
pub use document::{
    Document, Object, ObjectId, ObjectKind, CYLINDER_HEIGHT, CYLINDER_RADIUS, CYLINDER_SEGMENTS,
    DEFAULT_BOX,
};
pub use mesh::Mesh;
pub use ray::{ray_plane_y, Ray};
pub use stl::{document_to_stl, import_stl, mesh_from_stl, mesh_to_stl_binary, StlError};

/// Build plate edge length, in millimetres. The plate is centered on the origin.
pub const PLATE_SIZE_MM: f32 = 256.0;
/// Fine grid spacing on the build plate.
pub const GRID_MINOR_MM: f32 = 10.0;
/// Bold grid spacing on the build plate.
pub const GRID_MAJOR_MM: f32 = 50.0;

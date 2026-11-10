//! PrintCaddy document model. This crate has no graphics or browser dependencies.

mod boolean;
mod color;
mod document;
mod history;
mod mesh;
mod project;
mod ray;
mod sketch;
mod solids;
mod stl;
mod updates;

pub use boolean::{boolean, mesh_volume, BooleanError, BooleanOp};
pub use color::{Color, DEFAULT_MODEL_COLOR};
pub use document::{
    Document, GroupError, Object, ObjectId, ObjectKind, CYLINDER_HEIGHT, CYLINDER_RADIUS,
    CYLINDER_SEGMENTS, DEFAULT_BOX, DEFAULT_SNAP_MM,
};
pub use history::History;
pub use mesh::Mesh;
pub use project::{document_from_json, document_to_json, ProjectError};
pub use ray::{ray_plane_y, Ray};
pub use sketch::{build_sketch, is_closed, seat_sketch, SketchDef, SketchError, SketchOp};
pub use solids::{filleted_box, FilletError, Shape};
pub use stl::{document_to_stl, import_stl, mesh_from_stl, mesh_to_stl_binary, StlError};
pub use updates::{build_name, compare_builds, UpdateReport};

/// Round a millimetre measure onto the snap grid. A step of 0 leaves the value alone.
pub fn snap_mm(value: f32, step: f32) -> f32 {
    if step <= 1e-6 {
        value
    } else {
        (value / step).round() * step
    }
}

/// Build plate edge length, in millimetres. The plate is centered on the origin.
pub const PLATE_SIZE_MM: f32 = 256.0;
/// Fine grid spacing on the build plate.
pub const GRID_MINOR_MM: f32 = 10.0;
/// Bold grid spacing on the build plate.
pub const GRID_MAJOR_MM: f32 = 50.0;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use glam::{Mat4, Vec3};
use printcaddy_core::{Document, Object, GRID_MAJOR_MM, GRID_MINOR_MM, PLATE_SIZE_MM};

#[derive(Clone)]
pub struct UploadedMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
}

#[derive(Clone)]
pub struct DrawObject {
    pub id: u64,
    pub revision: u64,
    pub color: [f32; 3],
    pub selected: bool,
    pub alpha: f32,
    pub model: [f32; 16],
    pub mesh: Arc<UploadedMesh>,
}

#[derive(Clone)]
pub struct DrawFrame {
    pub view_proj: [f32; 16],
    pub objects: Vec<DrawObject>,
    pub grid: Arc<GridLines>,
}

pub struct GridLines {
    pub vertices: Vec<f32>,
}

impl GridLines {
    pub fn build() -> Self {
        let half = PLATE_SIZE_MM * 0.5;
        let minor = [0.23, 0.31, 0.36];
        let major = [0.42, 0.53, 0.60];
        let border = [0.62, 0.72, 0.78];
        let mut vertices = Vec::new();
        let mut line = |a: [f32; 3], b: [f32; 3], color: [f32; 3]| {
            vertices.extend_from_slice(&[a[0], a[1], a[2], color[0], color[1], color[2]]);
            vertices.extend_from_slice(&[b[0], b[1], b[2], color[0], color[1], color[2]]);
        };
        let major_every = (GRID_MAJOR_MM / GRID_MINOR_MM).round() as i32;
        // Grid lines sit on multiples of the minor step. The plate edge is its own rectangle,
        // because 256 mm is not a multiple of 10 mm.
        let mut cursor = (-half / GRID_MINOR_MM).ceil() * GRID_MINOR_MM;
        while cursor < half - 0.01 {
            let steps = (cursor / GRID_MINOR_MM).round() as i32;
            let color = if steps % major_every == 0 {
                major
            } else {
                minor
            };
            line([-half, 0.0, cursor], [half, 0.0, cursor], color);
            line([cursor, 0.0, -half], [cursor, 0.0, half], color);
            cursor += GRID_MINOR_MM;
        }
        line([-half, 0.0, -half], [half, 0.0, -half], border);
        line([half, 0.0, -half], [half, 0.0, half], border);
        line([half, 0.0, half], [-half, 0.0, half], border);
        line([-half, 0.0, half], [-half, 0.0, -half], border);
        let len = 36.0;
        let y = 0.2;
        line([0.0, y, 0.0], [len, y, 0.0], [0.86, 0.28, 0.28]);
        line([0.0, y, 0.0], [0.0, y + len, 0.0], [0.30, 0.72, 0.38]);
        line([0.0, y, 0.0], [0.0, y, len], [0.28, 0.52, 0.90]);
        Self { vertices }
    }
}

static GRID: LazyLock<Arc<GridLines>> = LazyLock::new(|| Arc::new(GridLines::build()));

pub fn mesh_cache_for(
    cache: &mut HashMap<(u64, u64), Arc<UploadedMesh>>,
    object: &Object,
) -> Arc<UploadedMesh> {
    let key = (object.id.0, object.revision);
    cache
        .entry(key)
        .or_insert_with(|| {
            let mesh = object.local_mesh();
            Arc::new(UploadedMesh {
                positions: mesh.positions.iter().flat_map(|p| *p).collect(),
                normals: mesh.normals.iter().flat_map(|p| *p).collect(),
                indices: mesh.indices.clone(),
            })
        })
        .clone()
}

pub fn draw_frame(
    doc: &Document,
    view_proj: Mat4,
    cache: &mut HashMap<(u64, u64), Arc<UploadedMesh>>,
) -> DrawFrame {
    let live: std::collections::HashSet<(u64, u64)> =
        doc.objects().iter().map(|o| (o.id.0, o.revision)).collect();
    cache.retain(|key, _| live.contains(key));
    let objects = doc
        .objects()
        .iter()
        .filter(|object| doc.is_shown(object.id))
        .map(|object| {
            let mut color = object.color.to_rgb_f32();
            if object.hole {
                color = [
                    color[0] * 0.35 + 0.20,
                    color[1] * 0.45 + 0.35,
                    color[2] * 0.45 + 0.62,
                ];
            }
            DrawObject {
                id: object.id.0,
                revision: object.revision,
                color,
                selected: doc.is_selected(object.id),
                alpha: doc.display_alpha(object.id),
                model: doc.display_matrix(object.id).to_cols_array(),
                mesh: mesh_cache_for(cache, object),
            }
        })
        .collect();
    DrawFrame {
        view_proj: view_proj.to_cols_array(),
        objects,
        grid: GRID.clone(),
    }
}

pub fn screen_ray(rect: egui::Rect, pos: egui::Pos2, view_proj: Mat4) -> printcaddy_core::Ray {
    let x = ((pos.x - rect.min.x) / rect.width().max(1.0)) * 2.0 - 1.0;
    let y = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)) * 2.0;
    let inv = view_proj.inverse();
    let near = inv.project_point3(Vec3::new(x, y, -1.0));
    let far = inv.project_point3(Vec3::new(x, y, 1.0));
    printcaddy_core::Ray::new(near, far - near)
}

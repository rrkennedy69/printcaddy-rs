use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use printcaddy_core::{Document, Object, GRID_MAJOR_MM, GRID_MINOR_MM, PLATE_SIZE_MM};

/// Mesh ids at or above this value are the filled beds. The painter draws them
/// before the grid lines so the lines stay on top.
pub const PLATE_MESH_ID: u64 = 0xFFFF_0000_0000_0000;

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
    /// 1 draws the colour flat. The build plate uses this so lighting cannot wash it out.
    pub unlit: f32,
    pub model: [f32; 16],
    pub mesh: Arc<UploadedMesh>,
}

#[derive(Clone)]
pub struct DrawFrame {
    pub view_proj: [f32; 16],
    pub objects: Vec<DrawObject>,
    pub grid: Arc<GridLines>,
    pub clear: [f32; 4],
    /// Hide mesh fragments above this world Y. A huge value leaves the model whole.
    pub clip_y: f32,
}

pub struct GridLines {
    pub vertices: Vec<f32>,
}

impl GridLines {
    /// One grid per plate centre. The axis triad stays on the plate at the origin.
    pub fn plates(
        minor: [f32; 3],
        major: [f32; 3],
        border: [f32; 3],
        centers: &[(f32, f32)],
    ) -> Self {
        let half = PLATE_SIZE_MM * 0.5;
        let mut vertices = Vec::new();
        let centers = if centers.is_empty() {
            vec![(0.0, 0.0)]
        } else {
            centers.to_vec()
        };
        for (ox, oz) in centers {
            let mut line = |a: [f32; 3], b: [f32; 3], color: [f32; 3]| {
                vertices.extend_from_slice(&[a[0], a[1], a[2], color[0], color[1], color[2]]);
                vertices.extend_from_slice(&[b[0], b[1], b[2], color[0], color[1], color[2]]);
            };
            let fine = [
                (minor[0] + 0.92) * 0.5,
                (minor[1] + 0.93) * 0.5,
                (minor[2] + 0.94) * 0.5,
            ];
            let mut fine_cursor = (-half / 5.0).ceil() * 5.0;
            while fine_cursor < half - 0.01 {
                line(
                    [ox - half, 0.0, oz + fine_cursor],
                    [ox + half, 0.0, oz + fine_cursor],
                    fine,
                );
                line(
                    [ox + fine_cursor, 0.0, oz - half],
                    [ox + fine_cursor, 0.0, oz + half],
                    fine,
                );
                fine_cursor += 5.0;
            }
            let major_every = (GRID_MAJOR_MM / GRID_MINOR_MM).round() as i32;
            let mut cursor = (-half / GRID_MINOR_MM).ceil() * GRID_MINOR_MM;
            while cursor < half - 0.01 {
                let steps = (cursor / GRID_MINOR_MM).round() as i32;
                let color = if steps % major_every == 0 {
                    major
                } else {
                    minor
                };
                line(
                    [ox - half, 0.0, oz + cursor],
                    [ox + half, 0.0, oz + cursor],
                    color,
                );
                line(
                    [ox + cursor, 0.0, oz - half],
                    [ox + cursor, 0.0, oz + half],
                    color,
                );
                cursor += GRID_MINOR_MM;
            }
            line(
                [ox - half, 0.0, oz - half],
                [ox + half, 0.0, oz - half],
                border,
            );
            line(
                [ox + half, 0.0, oz - half],
                [ox + half, 0.0, oz + half],
                border,
            );
            line(
                [ox + half, 0.0, oz + half],
                [ox - half, 0.0, oz + half],
                border,
            );
            line(
                [ox - half, 0.0, oz + half],
                [ox - half, 0.0, oz - half],
                border,
            );
            if ox.abs() < 0.1 && oz.abs() < 0.1 {
                // Live triad: front-left corner, X right, Y toward the back, Z up. Length 24 mm on a 200 mm bed.
                let len = 24.0;
                let y = 0.08;
                let x0 = ox - half;
                let z0 = oz + half;
                line([x0, y, z0], [x0 + len, y, z0], [0.898, 0.282, 0.302]);
                line([x0, y, z0], [x0, y, z0 - len], [0.247, 0.682, 0.353]);
                line([x0, y, z0], [x0, y + len, z0], [0.231, 0.510, 0.965]);
            }
        }
        Self { vertices }
    }
}

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

fn plate_quad() -> Arc<UploadedMesh> {
    let half = PLATE_SIZE_MM * 0.5;
    Arc::new(UploadedMesh {
        positions: vec![
            -half, 0.0, -half, half, 0.0, -half, half, 0.0, half, -half, 0.0, half,
        ],
        normals: vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
        indices: vec![0, 1, 2, 0, 2, 3],
    })
}

pub fn draw_frame(
    doc: &Document,
    view_proj: Mat4,
    cache: &mut HashMap<(u64, u64), Arc<UploadedMesh>>,
    grid: Arc<GridLines>,
    clear: [f32; 4],
    plate_color: [f32; 3],
    xray: bool,
    clip_y: f32,
) -> DrawFrame {
    let live: std::collections::HashSet<(u64, u64)> =
        doc.objects().iter().map(|o| (o.id.0, o.revision)).collect();
    cache.retain(|key, _| live.contains(key));
    let bed = plate_quad();
    let mut objects: Vec<DrawObject> = doc
        .plate_layouts()
        .iter()
        .enumerate()
        .map(|(index, plate)| DrawObject {
            id: PLATE_MESH_ID + index as u64,
            revision: 1,
            color: plate_color,
            selected: false,
            alpha: 1.0,
            unlit: 1.0,
            model: Mat4::from_translation(Vec3::new(plate.center_x, -0.05, plate.center_z))
                .to_cols_array(),
            mesh: Arc::clone(&bed),
        })
        .collect();
    objects.extend(doc.objects().iter()
        .filter(|object| doc.is_shown(object.id))
        .map(|object| {
            let mut color = object.color.to_rgb_f32();
            let mut alpha = doc.display_alpha(object.id);
            if object.hole {
                color = [
                    color[0] * 0.35 + 0.20,
                    color[1] * 0.45 + 0.35,
                    color[2] * 0.45 + 0.62,
                ];
            }
            if xray {
                color = [
                    color[0] * 0.35 + 0.55,
                    color[1] * 0.25 + 0.45,
                    color[2] * 0.35 + 0.75,
                ];
                alpha = alpha.min(0.34);
            }
            DrawObject {
                id: object.id.0,
                revision: object.revision,
                color,
                selected: doc.is_selected(object.id),
                alpha,
                unlit: 0.0,
                model: doc.display_matrix(object.id).to_cols_array(),
                mesh: mesh_cache_for(cache, object),
            }
        }));
    DrawFrame {
        view_proj: view_proj.to_cols_array(),
        objects,
        grid,
        clear,
        clip_y,
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

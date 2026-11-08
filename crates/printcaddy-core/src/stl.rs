use glam::Vec3;

use crate::document::{Document, ObjectId};
use crate::mesh::{face_normal, Mesh};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StlError {
    Empty,
    Truncated,
    BadAscii,
}

pub fn mesh_to_stl_binary(mesh: &Mesh) -> Vec<u8> {
    let triangles = mesh.triangle_count();
    let mut out = vec![0u8; 84 + triangles * 50];
    out[0..5].copy_from_slice(b"Print");
    out[80..84].copy_from_slice(&(triangles as u32).to_le_bytes());
    for (tri, chunk) in mesh
        .indices
        .chunks_exact(3)
        .zip(out[84..].chunks_exact_mut(50))
    {
        let a = Vec3::from_array(mesh.positions[tri[0] as usize]);
        let b = Vec3::from_array(mesh.positions[tri[1] as usize]);
        let c = Vec3::from_array(mesh.positions[tri[2] as usize]);
        let n = face_normal(a, b, c);
        write_f32s(
            chunk,
            &[n.x, n.y, n.z, a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z],
        );
    }
    out
}

pub fn mesh_from_stl(bytes: &[u8]) -> Result<Mesh, StlError> {
    if bytes.len() < 15 {
        return Err(StlError::Empty);
    }
    if looks_like_ascii(bytes) {
        mesh_from_ascii(bytes)
    } else {
        mesh_from_binary(bytes)
    }
}

/// Binary STL of every body, or only the selection when one exists and `only_selected` is set.
pub fn document_to_stl(doc: &Document, only_selected: bool) -> Vec<u8> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let selected = doc.selected_ids();
    for object in doc.objects() {
        if only_selected && !selected.is_empty() && !selected.contains(&object.id) {
            continue;
        }
        let mesh = object.world_mesh();
        let base = positions.len() as u32;
        positions.extend(mesh.positions);
        normals.extend(mesh.normals);
        indices.extend(mesh.indices.into_iter().map(|i| i + base));
    }
    mesh_to_stl_binary(&Mesh {
        positions,
        normals,
        indices,
    })
}

pub fn import_stl(doc: &mut Document, name: &str, bytes: &[u8]) -> Result<ObjectId, StlError> {
    let mesh = mesh_from_stl(bytes)?;
    if mesh.triangle_count() == 0 {
        return Err(StlError::Empty);
    }
    Ok(doc.add_mesh(name, mesh))
}

fn looks_like_ascii(bytes: &[u8]) -> bool {
    let trimmed = trim_ascii_start(bytes);
    if !trimmed.starts_with(b"solid") {
        return false;
    }
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        let expected = 84usize.saturating_add(count.saturating_mul(50));
        if expected == bytes.len() && !bytes.windows(5).any(|w| w.eq_ignore_ascii_case(b"facet")) {
            return false;
        }
    }
    true
}

fn mesh_from_binary(bytes: &[u8]) -> Result<Mesh, StlError> {
    if bytes.len() < 84 {
        return Err(StlError::Truncated);
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let expected = 84usize.saturating_add(count.saturating_mul(50));
    if bytes.len() < expected {
        return Err(StlError::Truncated);
    }
    let mut positions = Vec::with_capacity(count * 3);
    let mut normals = Vec::with_capacity(count * 3);
    let mut indices = Vec::with_capacity(count * 3);
    for i in 0..count {
        let start = 84 + i * 50;
        let n = read_vec3(&bytes[start..start + 12]);
        let a = read_vec3(&bytes[start + 12..start + 24]);
        let b = read_vec3(&bytes[start + 24..start + 36]);
        let c = read_vec3(&bytes[start + 36..start + 48]);
        let normal = if n.length_squared() < 1e-12 {
            face_normal(a, b, c)
        } else {
            n.normalize()
        };
        let base = positions.len() as u32;
        for p in [a, b, c] {
            positions.push(p.to_array());
            normals.push(normal.to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Ok(Mesh {
        positions,
        normals,
        indices,
    })
}

fn mesh_from_ascii(bytes: &[u8]) -> Result<Mesh, StlError> {
    let text = std::str::from_utf8(bytes).map_err(|_| StlError::BadAscii)?;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut verts = Vec::new();
    let mut normal = Vec3::Y;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("facet normal") {
            normal = parse_vec3(rest).unwrap_or(Vec3::Y);
        } else if let Some(rest) = line.strip_prefix("vertex") {
            let v = parse_vec3(rest).ok_or(StlError::BadAscii)?;
            verts.push(v);
            if verts.len() == 3 {
                let n = if normal.length_squared() < 1e-12 {
                    face_normal(verts[0], verts[1], verts[2])
                } else {
                    normal.normalize()
                };
                let base = positions.len() as u32;
                for p in &verts {
                    positions.push(p.to_array());
                    normals.push(n.to_array());
                }
                indices.extend_from_slice(&[base, base + 1, base + 2]);
                verts.clear();
            }
        }
    }
    if positions.is_empty() {
        return Err(StlError::Empty);
    }
    Ok(Mesh {
        positions,
        normals,
        indices,
    })
}

fn parse_vec3(text: &str) -> Option<Vec3> {
    let mut nums = text.split_whitespace();
    let x: f32 = nums.next()?.parse().ok()?;
    let y: f32 = nums.next()?.parse().ok()?;
    let z: f32 = nums.next()?.parse().ok()?;
    Some(Vec3::new(x, y, z))
}

fn read_vec3(bytes: &[u8]) -> Vec3 {
    Vec3::new(
        read_f32(&bytes[0..4]),
        read_f32(&bytes[4..8]),
        read_f32(&bytes[8..12]),
    )
}

fn read_f32(bytes: &[u8]) -> f32 {
    f32::from_le_bytes(bytes.try_into().unwrap())
}

fn write_f32s(dest: &mut [u8], values: &[f32]) {
    for (slot, value) in dest.chunks_exact_mut(4).zip(values) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

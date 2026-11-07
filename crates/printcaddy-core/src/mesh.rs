use glam::Vec3;

/// Indexed triangle mesh in millimetres. Local meshes sit with their bottom on Y = 0
/// and their footprint centered on X and Z.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for p in &self.positions {
            let v = Vec3::from_array(*p);
            min = min.min(v);
            max = max.max(v);
        }
        if self.positions.is_empty() {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (min, max)
        }
    }

    pub fn translated(&self, by: Vec3) -> Self {
        Self {
            positions: self
                .positions
                .iter()
                .map(|p| (Vec3::from_array(*p) + by).to_array())
                .collect(),
            normals: self.normals.clone(),
            indices: self.indices.clone(),
        }
    }
}

pub fn box_mesh(width: f32, depth: f32, height: f32) -> Mesh {
    let hx = width * 0.5;
    let hz = depth * 0.5;
    let y0 = 0.0;
    let y1 = height;
    // Each face has its own four corners so the lighting stays crisp on the edges.
    let faces: [[([f32; 3], [f32; 3]); 4]; 6] = [
        // top
        [
            ([-hx, y1, -hz], [0.0, 1.0, 0.0]),
            ([hx, y1, -hz], [0.0, 1.0, 0.0]),
            ([hx, y1, hz], [0.0, 1.0, 0.0]),
            ([-hx, y1, hz], [0.0, 1.0, 0.0]),
        ],
        // bottom
        [
            ([-hx, y0, hz], [0.0, -1.0, 0.0]),
            ([hx, y0, hz], [0.0, -1.0, 0.0]),
            ([hx, y0, -hz], [0.0, -1.0, 0.0]),
            ([-hx, y0, -hz], [0.0, -1.0, 0.0]),
        ],
        // +X
        [
            ([hx, y0, -hz], [1.0, 0.0, 0.0]),
            ([hx, y0, hz], [1.0, 0.0, 0.0]),
            ([hx, y1, hz], [1.0, 0.0, 0.0]),
            ([hx, y1, -hz], [1.0, 0.0, 0.0]),
        ],
        // -X
        [
            ([-hx, y0, hz], [-1.0, 0.0, 0.0]),
            ([-hx, y0, -hz], [-1.0, 0.0, 0.0]),
            ([-hx, y1, -hz], [-1.0, 0.0, 0.0]),
            ([-hx, y1, hz], [-1.0, 0.0, 0.0]),
        ],
        // +Z
        [
            ([hx, y0, hz], [0.0, 0.0, 1.0]),
            ([-hx, y0, hz], [0.0, 0.0, 1.0]),
            ([-hx, y1, hz], [0.0, 0.0, 1.0]),
            ([hx, y1, hz], [0.0, 0.0, 1.0]),
        ],
        // -Z
        [
            ([-hx, y0, -hz], [0.0, 0.0, -1.0]),
            ([hx, y0, -hz], [0.0, 0.0, -1.0]),
            ([hx, y1, -hz], [0.0, 0.0, -1.0]),
            ([-hx, y1, -hz], [0.0, 0.0, -1.0]),
        ],
    ];
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for face in faces {
        let base = positions.len() as u32;
        for (p, n) in face {
            positions.push(p);
            normals.push(n);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh {
        positions,
        normals,
        indices,
    }
}

pub fn cylinder_mesh(radius: f32, height: f32, segments: u32) -> Mesh {
    let segments = segments.max(3);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let push = |positions: &mut Vec<[f32; 3]>,
                normals: &mut Vec<[f32; 3]>,
                p: [f32; 3],
                n: [f32; 3]|
     -> u32 {
        let id = positions.len() as u32;
        positions.push(p);
        normals.push(n);
        id
    };

    for i in 0..segments {
        let a0 = (i as f32) / (segments as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32) / (segments as f32) * std::f32::consts::TAU;
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());
        let x0 = c0 * radius;
        let z0 = s0 * radius;
        let x1 = c1 * radius;
        let z1 = s1 * radius;
        let n0 = [c0, 0.0, s0];
        let n1 = [c1, 0.0, s1];
        let b0 = push(&mut positions, &mut normals, [x0, 0.0, z0], n0);
        let b1 = push(&mut positions, &mut normals, [x1, 0.0, z1], n1);
        let t1 = push(&mut positions, &mut normals, [x1, height, z1], n1);
        let t0 = push(&mut positions, &mut normals, [x0, height, z0], n0);
        indices.extend_from_slice(&[b0, b1, t1, b0, t1, t0]);

        let top_c = push(
            &mut positions,
            &mut normals,
            [0.0, height, 0.0],
            [0.0, 1.0, 0.0],
        );
        let top_a = push(
            &mut positions,
            &mut normals,
            [x0, height, z0],
            [0.0, 1.0, 0.0],
        );
        let top_b = push(
            &mut positions,
            &mut normals,
            [x1, height, z1],
            [0.0, 1.0, 0.0],
        );
        indices.extend_from_slice(&[top_c, top_a, top_b]);

        let bot_c = push(
            &mut positions,
            &mut normals,
            [0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        );
        let bot_a = push(
            &mut positions,
            &mut normals,
            [x0, 0.0, z0],
            [0.0, -1.0, 0.0],
        );
        let bot_b = push(
            &mut positions,
            &mut normals,
            [x1, 0.0, 0.0 * 0.0 + z1],
            [0.0, -1.0, 0.0],
        );
        indices.extend_from_slice(&[bot_c, bot_b, bot_a]);
    }

    Mesh {
        positions,
        normals,
        indices,
    }
}

pub(crate) fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    if n.length_squared() < 1e-12 {
        Vec3::Y
    } else {
        n.normalize()
    }
}

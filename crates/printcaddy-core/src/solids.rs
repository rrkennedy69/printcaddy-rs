//! Simple solids from the current PrintCaddy shape library, at that app's starting sizes.
//!
//! Text and name tags are not here: they need a font. Mechanical parts (threads, gears,
//! hinges, and the rest) are later rounds.

use std::f32::consts::TAU;

use glam::Vec3;

use crate::mesh::{box_mesh, Mesh};

pub const CYLINDER_SEGMENTS: u32 = 32;

/// A drop-in solid. Sizes are the library defaults, in millimetres: width, depth, height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    Box,
    RoundedBox,
    Cylinder,
    Capsule,
    Ellipse,
    Polygon,
    Sphere,
    Cone,
    Pyramid,
    Wedge,
    RoundRoof,
    HalfSphere,
    Paraboloid,
    Torus,
    Tube,
    Star,
    Heart,
}

impl Shape {
    pub const ALL: [Shape; 17] = [
        Shape::Box,
        Shape::RoundedBox,
        Shape::Cylinder,
        Shape::Capsule,
        Shape::Ellipse,
        Shape::Polygon,
        Shape::Sphere,
        Shape::Cone,
        Shape::Pyramid,
        Shape::Wedge,
        Shape::RoundRoof,
        Shape::HalfSphere,
        Shape::Paraboloid,
        Shape::Torus,
        Shape::Tube,
        Shape::Star,
        Shape::Heart,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shape::Box => "Box",
            Shape::RoundedBox => "Rounded box",
            Shape::Cylinder => "Cylinder",
            Shape::Capsule => "Capsule",
            Shape::Ellipse => "Ellipse",
            Shape::Polygon => "Polygon",
            Shape::Sphere => "Sphere",
            Shape::Cone => "Cone",
            Shape::Pyramid => "Pyramid",
            Shape::Wedge => "Wedge",
            Shape::RoundRoof => "Round roof",
            Shape::HalfSphere => "Half sphere",
            Shape::Paraboloid => "Paraboloid",
            Shape::Torus => "Torus",
            Shape::Tube => "Tube",
            Shape::Star => "Star",
            Shape::Heart => "Heart",
        }
    }

    /// Starting width, depth, and height, matching `shapeAssetDefaultDimensions`.
    pub fn default_size(self) -> (f32, f32, f32) {
        match self {
            Shape::Box | Shape::Cylinder | Shape::Cone | Shape::Pyramid | Shape::Wedge => {
                (20.0, 20.0, 20.0)
            }
            Shape::RoundedBox => (40.0, 30.0, 20.0),
            Shape::Capsule => (40.0, 20.0, 20.0),
            Shape::Ellipse => (26.0, 16.0, 20.0),
            Shape::Polygon => {
                let (aw, ad) = polygon_aspect(6);
                let longest = aw.max(ad);
                (20.0 * aw / longest, 20.0 * ad / longest, 20.0)
            }
            Shape::Sphere => (22.0, 22.0, 22.0),
            Shape::RoundRoof => (20.0, 20.0, 10.0),
            Shape::HalfSphere => (22.0, 22.0, 11.0),
            Shape::Paraboloid => (22.0, 22.0, 20.0),
            Shape::Torus => (22.0, 22.0, 5.0),
            Shape::Tube => (20.0, 20.0, 20.0),
            Shape::Star => (40.0, 40.0, 10.0),
            Shape::Heart => (40.0, 40.0, 10.0),
        }
    }
}

pub fn shape_mesh(shape: Shape, width: f32, depth: f32, height: f32) -> Mesh {
    let (width, depth, height) = (width.max(0.2), depth.max(0.2), height.max(0.2));
    match shape {
        Shape::Box => box_mesh(width, depth, height),
        Shape::RoundedBox => rounded_box(width, depth, height, 5.0),
        Shape::Cylinder => elliptical_cylinder(width, depth, height, CYLINDER_SEGMENTS),
        Shape::Ellipse => elliptical_cylinder(width, depth, height, CYLINDER_SEGMENTS),
        Shape::Capsule => capsule(width, depth, height),
        Shape::Polygon => extrude(&polygon_points(6, width, depth), height),
        Shape::Sphere => ellipsoid(width, depth, height, 16, 32, false),
        Shape::HalfSphere => ellipsoid(width, depth, height * 2.0, 12, 32, true),
        Shape::Cone => cone(width, depth, height, CYLINDER_SEGMENTS),
        Shape::Pyramid => pyramid(width, depth, height),
        Shape::Wedge => wedge(width, depth, height),
        Shape::RoundRoof => round_roof(width, depth, height),
        Shape::Paraboloid => paraboloid(width, depth, height, 24),
        Shape::Torus => torus(width, depth, height),
        Shape::Tube => tube(width, depth, height, 4.0),
        Shape::Star => extrude(&star_points(5, 0.5, width, depth), height),
        Shape::Heart => extrude(&heart_points(width, depth), height),
    }
}

fn polygon_aspect(sides: u32) -> (f32, f32) {
    let n = sides.max(3);
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for i in 0..n {
        let angle = (i as f32) / (n as f32) * TAU;
        let x = angle.sin();
        let z = angle.cos();
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    (max_x - min_x, max_z - min_z)
}

fn polygon_points(sides: u32, width: f32, depth: f32) -> Vec<[f32; 2]> {
    let n = sides.max(3);
    let (aw, ad) = polygon_aspect(n);
    (0..n)
        .map(|i| {
            let angle = (i as f32) / (n as f32) * TAU;
            [angle.sin() / aw * width, angle.cos() / ad * depth]
        })
        .collect()
}

fn star_points(points: u32, inner_ratio: f32, width: f32, depth: f32) -> Vec<[f32; 2]> {
    let n = points.max(3);
    let mut raw = Vec::new();
    for i in 0..n * 2 {
        let angle = (i as f32) / (n as f32 * 2.0) * TAU - std::f32::consts::FRAC_PI_2;
        let radius = if i % 2 == 0 {
            1.0
        } else {
            inner_ratio.clamp(0.05, 0.95)
        };
        raw.push([angle.cos() * radius, angle.sin() * radius]);
    }
    fit_polygon(&raw, width, depth)
}

fn heart_points(width: f32, depth: f32) -> Vec<[f32; 2]> {
    let steps = 48;
    let mut raw = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = (i as f32) / (steps as f32) * TAU;
        let x = 16.0 * t.sin().powi(3);
        let z = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
        raw.push([x, z]);
    }
    fit_polygon(&raw, width, depth)
}

fn fit_polygon(raw: &[[f32; 2]], width: f32, depth: f32) -> Vec<[f32; 2]> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for p in raw {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_z = min_z.min(p[1]);
        max_z = max_z.max(p[1]);
    }
    let sx = width / (max_x - min_x).max(1e-4);
    let sz = depth / (max_z - min_z).max(1e-4);
    let cx = (min_x + max_x) * 0.5;
    let cz = (min_z + max_z) * 0.5;
    raw.iter()
        .map(|p| [(p[0] - cx) * sx, (p[1] - cz) * sz])
        .collect()
}

struct Build {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl Build {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn vertex(&mut self, p: [f32; 3], n: [f32; 3]) -> u32 {
        let id = self.positions.len() as u32;
        self.positions.push(p);
        self.normals.push(n);
        id
    }

    fn tri(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        let n = normal(a, b, c);
        let ia = self.vertex(a, n);
        let ib = self.vertex(b, n);
        let ic = self.vertex(c, n);
        self.indices.extend_from_slice(&[ia, ib, ic]);
    }

    fn mesh(self) -> Mesh {
        Mesh {
            positions: self.positions,
            normals: self.normals,
            indices: self.indices,
        }
    }
}

fn normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ux = b[0] - a[0];
    let uy = b[1] - a[1];
    let uz = b[2] - a[2];
    let vx = c[0] - a[0];
    let vy = c[1] - a[1];
    let vz = c[2] - a[2];
    let n = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < 1e-8 {
        [0.0, 1.0, 0.0]
    } else {
        [n[0] / len, n[1] / len, n[2] / len]
    }
}

fn extrude(loop_xz: &[[f32; 2]], height: f32) -> Mesh {
    let mut b = Build::new();
    let n = loop_xz.len();
    if n < 3 {
        return b.mesh();
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (x0, z0) = (loop_xz[i][0], loop_xz[i][1]);
        let (x1, z1) = (loop_xz[j][0], loop_xz[j][1]);
        b.tri([x0, 0.0, z0], [x1, 0.0, z1], [x1, height, z1]);
        b.tri([x0, 0.0, z0], [x1, height, z1], [x0, height, z0]);
    }
    let (cx, cz) = centroid(loop_xz);
    for i in 0..n {
        let j = (i + 1) % n;
        b.tri(
            [cx, height, cz],
            [loop_xz[i][0], height, loop_xz[i][1]],
            [loop_xz[j][0], height, loop_xz[j][1]],
        );
        b.tri(
            [cx, 0.0, cz],
            [loop_xz[j][0], 0.0, loop_xz[j][1]],
            [loop_xz[i][0], 0.0, loop_xz[i][1]],
        );
    }
    b.mesh()
}

fn centroid(points: &[[f32; 2]]) -> (f32, f32) {
    let n = points.len() as f32;
    let (sx, sz) = points
        .iter()
        .fold((0.0, 0.0), |(x, z), p| (x + p[0], z + p[1]));
    (sx / n, sz / n)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilletError {
    NotABox,
    RadiusTooSmall,
    RadiusTooLarge,
}

impl std::fmt::Display for FilletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            FilletError::NotABox => "Fillet only rounds the edges of a box.",
            FilletError::RadiusTooSmall => "The fillet radius must be at least 0.2 mm.",
            FilletError::RadiusTooLarge => {
                "The fillet radius must be at least 0.2 mm shorter than half the smallest side."
            }
        };
        f.write_str(text)
    }
}

/// Round every edge of a box. The radius has to be at least 0.2 mm and at least
/// 0.2 mm short of half the smallest side. Top, bottom, and vertical edges are all rounded.
pub fn filleted_box(width: f32, depth: f32, height: f32, radius: f32) -> Result<Mesh, FilletError> {
    if radius < 0.2 {
        return Err(FilletError::RadiusTooSmall);
    }
    let limit = width.min(depth).min(height) * 0.5 - 0.2;
    if radius > limit {
        return Err(FilletError::RadiusTooLarge);
    }
    Ok(rounded_edges(width, depth, height, radius))
}

fn rounded_edges(width: f32, depth: f32, height: f32, radius: f32) -> Mesh {
    let center = Vec3::new(0.0, height * 0.5, 0.0);
    let half = Vec3::new(width * 0.5, height * 0.5, depth * 0.5);
    let seg = 4i32;
    let faces = [
        (
            Vec3::new(-half.x, half.y, -half.z),
            Vec3::new(width, 0.0, 0.0),
            Vec3::new(0.0, 0.0, depth),
        ),
        (
            Vec3::new(-half.x, -half.y, half.z),
            Vec3::new(width, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -depth),
        ),
        (
            Vec3::new(half.x, -half.y, -half.z),
            Vec3::new(0.0, 0.0, depth),
            Vec3::new(0.0, height, 0.0),
        ),
        (
            Vec3::new(-half.x, -half.y, half.z),
            Vec3::new(0.0, 0.0, -depth),
            Vec3::new(0.0, height, 0.0),
        ),
        (
            Vec3::new(half.x, -half.y, half.z),
            Vec3::new(-width, 0.0, 0.0),
            Vec3::new(0.0, height, 0.0),
        ),
        (
            Vec3::new(-half.x, -half.y, -half.z),
            Vec3::new(width, 0.0, 0.0),
            Vec3::new(0.0, height, 0.0),
        ),
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for (origin, axis_u, axis_v) in faces {
        let mut grid = Vec::new();
        for iv in 0..=seg {
            for iu in 0..=seg {
                let u = iu as f32 / seg as f32;
                let v = iv as f32 / seg as f32;
                let point = origin + axis_u * u + axis_v * v;
                grid.push(round_corner(point, half, radius) + center);
            }
        }
        let stride = seg + 1;
        for iv in 0..seg {
            for iu in 0..seg {
                let i00 = (iv * stride + iu) as usize;
                let i10 = i00 + 1;
                let i01 = i00 + stride as usize;
                let i11 = i01 + 1;
                push_rounded_tri(
                    &mut positions,
                    &mut normals,
                    &mut indices,
                    grid[i00],
                    grid[i10],
                    grid[i11],
                    center,
                );
                push_rounded_tri(
                    &mut positions,
                    &mut normals,
                    &mut indices,
                    grid[i00],
                    grid[i11],
                    grid[i01],
                    center,
                );
            }
        }
    }
    Mesh {
        positions,
        normals,
        indices,
    }
}

fn round_corner(point: Vec3, half: Vec3, radius: f32) -> Vec3 {
    let inner = half - Vec3::splat(radius);
    let ax = point.x.abs();
    let ay = point.y.abs();
    let az = point.z.abs();
    let dx = (ax - inner.x).max(0.0);
    let dy = (ay - inner.y).max(0.0);
    let dz = (az - inner.z).max(0.0);
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len <= 1.0e-8 {
        return point;
    }
    Vec3::new(
        point.x.signum() * (ax - dx + dx / len * radius),
        point.y.signum() * (ay - dy + dy / len * radius),
        point.z.signum() * (az - dz + dz / len * radius),
    )
}

fn push_rounded_tri(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    center: Vec3,
) {
    let mut normal = (b - a).cross(c - a);
    if normal.length_squared() < 1.0e-10 {
        return;
    }
    normal = normal.normalize();
    let mid = (a + b + c) / 3.0;
    if normal.dot(mid - center) < 0.0 {
        normal = -normal;
    }
    let base = positions.len() as u32;
    for point in [a, b, c] {
        positions.push(point.to_array());
        normals.push(normal.to_array());
    }
    indices.extend_from_slice(&[base, base + 1, base + 2]);
}

fn rounded_box(width: f32, depth: f32, height: f32, fillet: f32) -> Mesh {
    let r = fillet
        .min(width * 0.5 - 0.05)
        .min(depth * 0.5 - 0.05)
        .max(0.0);
    if r < 0.05 {
        return box_mesh(width, depth, height);
    }
    let hx = width * 0.5;
    let hz = depth * 0.5;
    let mut pts = Vec::new();
    let corners = [
        (hx - r, hz - r, 0.0),
        (-hx + r, hz - r, 1.0),
        (-hx + r, -hz + r, 2.0),
        (hx - r, -hz + r, 3.0),
    ];
    for (cx, cz, quad) in corners {
        for s in 0..=4 {
            let angle = (quad * 0.5 + s as f32 / 4.0 * 0.5) * std::f32::consts::PI;
            pts.push([cx + angle.cos() * r, cz + angle.sin() * r]);
        }
    }
    extrude(&pts, height)
}

fn elliptical_cylinder(width: f32, depth: f32, height: f32, segments: u32) -> Mesh {
    let mut b = Build::new();
    let segments = segments.max(3);
    for i in 0..segments {
        let a0 = (i as f32) / (segments as f32) * TAU;
        let a1 = ((i + 1) as f32) / (segments as f32) * TAU;
        let p0 = [a0.cos() * width * 0.5, 0.0, a0.sin() * depth * 0.5];
        let p1 = [a1.cos() * width * 0.5, 0.0, a1.sin() * depth * 0.5];
        let t0 = [p0[0], height, p0[2]];
        let t1 = [p1[0], height, p1[2]];
        b.tri(p0, p1, t1);
        b.tri(p0, t1, t0);
        b.tri([0.0, height, 0.0], t0, t1);
        b.tri([0.0, 0.0, 0.0], p1, p0);
    }
    b.mesh()
}

fn capsule(width: f32, depth: f32, height: f32) -> Mesh {
    let (length, thick) = if width >= depth {
        (width, depth)
    } else {
        (depth, width)
    };
    let radius = thick * 0.5;
    let straight = (length - thick).max(0.0) * 0.5;
    let mut pts = Vec::new();
    let steps = 10;
    for i in 0..=steps {
        let a = (i as f32) / (steps as f32) * std::f32::consts::PI;
        pts.push([straight + a.sin() * radius, a.cos() * radius]);
    }
    for i in 0..=steps {
        let a = (i as f32) / (steps as f32) * std::f32::consts::PI;
        pts.push([-straight - a.sin() * radius, -a.cos() * radius]);
    }
    if width < depth {
        for p in &mut pts {
            let (x, z) = (p[0], p[1]);
            p[0] = z;
            p[1] = x;
        }
    }
    extrude(&pts, height)
}

fn ellipsoid(
    width: f32,
    depth: f32,
    full_height: f32,
    stacks: u32,
    slices: u32,
    upper_only: bool,
) -> Mesh {
    let mut b = Build::new();
    let stacks = if upper_only {
        stacks.max(2)
    } else {
        stacks.max(2)
    };
    let slices = slices.max(3);
    let stack_count = if upper_only { stacks } else { stacks };
    let y_span = if upper_only {
        full_height * 0.5
    } else {
        full_height
    };
    for stack in 0..stack_count {
        let v0 = stack as f32 / stack_count as f32;
        let v1 = (stack + 1) as f32 / stack_count as f32;
        let phi0 = v0
            * if upper_only {
                std::f32::consts::FRAC_PI_2
            } else {
                std::f32::consts::PI
            };
        let phi1 = v1
            * if upper_only {
                std::f32::consts::FRAC_PI_2
            } else {
                std::f32::consts::PI
            };
        for slice in 0..slices {
            let u0 = slice as f32 / slices as f32 * TAU;
            let u1 = (slice + 1) as f32 / slices as f32 * TAU;
            let p = |phi: f32, theta: f32| -> [f32; 3] {
                let y = if upper_only {
                    y_span * phi.cos()
                } else {
                    y_span * 0.5 * phi.cos() + y_span * 0.5
                };
                [
                    width * 0.5 * phi.sin() * theta.cos(),
                    y,
                    depth * 0.5 * phi.sin() * theta.sin(),
                ]
            };
            let a = p(phi0, u0);
            let c = p(phi0, u1);
            let d = p(phi1, u1);
            let e = p(phi1, u0);
            if stack != 0 {
                b.tri(a, e, c);
            }
            if stack + 1 != stack_count || upper_only {
                b.tri(c, e, d);
            }
            if upper_only && stack + 1 == stack_count {
                b.tri([0.0, 0.0, 0.0], c, e);
            }
        }
    }
    b.mesh()
}

fn cone(width: f32, depth: f32, height: f32, segments: u32) -> Mesh {
    let mut b = Build::new();
    let segments = segments.max(3);
    let apex = [0.0, height, 0.0];
    for i in 0..segments {
        let a0 = (i as f32) / (segments as f32) * TAU;
        let a1 = ((i + 1) as f32) / (segments as f32) * TAU;
        let p0 = [a0.cos() * width * 0.5, 0.0, a0.sin() * depth * 0.5];
        let p1 = [a1.cos() * width * 0.5, 0.0, a1.sin() * depth * 0.5];
        b.tri(p0, p1, apex);
        b.tri([0.0, 0.0, 0.0], p1, p0);
    }
    b.mesh()
}

fn pyramid(width: f32, depth: f32, height: f32) -> Mesh {
    let mut b = Build::new();
    let hx = width * 0.5;
    let hz = depth * 0.5;
    let apex = [0.0, height, 0.0];
    let corners = [
        [-hx, 0.0, -hz],
        [hx, 0.0, -hz],
        [hx, 0.0, hz],
        [-hx, 0.0, hz],
    ];
    for i in 0..4 {
        b.tri(corners[i], corners[(i + 1) % 4], apex);
    }
    b.tri(corners[0], corners[3], corners[2]);
    b.tri(corners[0], corners[2], corners[1]);
    b.mesh()
}

fn wedge(width: f32, depth: f32, height: f32) -> Mesh {
    let mut b = Build::new();
    let hx = width * 0.5;
    let hz = depth * 0.5;
    let a = [-hx, 0.0, -hz];
    let c = [hx, 0.0, -hz];
    let d = [hx, 0.0, hz];
    let e = [-hx, 0.0, hz];
    let top_l = [-hx, height, -hz];
    let top_r = [hx, height, -hz];
    b.tri(a, c, top_r);
    b.tri(a, top_r, top_l);
    b.tri(a, top_l, e);
    b.tri(e, top_l, d);
    b.tri(d, top_l, top_r);
    b.tri(c, d, top_r);
    b.tri(a, e, d);
    b.tri(a, d, c);
    b.mesh()
}

fn round_roof(width: f32, depth: f32, height: f32) -> Mesh {
    let mut b = Build::new();
    let steps = 16;
    let hz = depth * 0.5;
    let mut profile = Vec::new();
    for i in 0..=steps {
        let t = (i as f32) / (steps as f32) * std::f32::consts::PI;
        profile.push([-width * 0.5 * t.cos(), height * t.sin()]);
    }
    for i in 0..steps {
        let (x0, y0) = (profile[i][0], profile[i][1]);
        let (x1, y1) = (profile[i + 1][0], profile[i + 1][1]);
        b.tri([x0, y0, -hz], [x1, y1, -hz], [x1, y1, hz]);
        b.tri([x0, y0, -hz], [x1, y1, hz], [x0, y0, hz]);
        b.tri([0.0, 0.0, -hz], [x0, y0, -hz], [x1, y1, -hz]);
        b.tri([0.0, 0.0, hz], [x1, y1, hz], [x0, y0, hz]);
    }
    b.mesh()
}

fn paraboloid(width: f32, depth: f32, height: f32, segments: u32) -> Mesh {
    let mut b = Build::new();
    let rings = 8;
    let segments = segments.max(3);
    for ring in 0..rings {
        let r0 = 1.0 - ring as f32 / rings as f32;
        let r1 = 1.0 - (ring + 1) as f32 / rings as f32;
        let y0 = height * (1.0 - r0 * r0);
        let y1 = height * (1.0 - r1 * r1);
        for i in 0..segments {
            let a0 = (i as f32) / (segments as f32) * TAU;
            let a1 = ((i + 1) as f32) / (segments as f32) * TAU;
            let p =
                |r: f32, y: f32, a: f32| [a.cos() * width * 0.5 * r, y, a.sin() * depth * 0.5 * r];
            b.tri(p(r0, y0, a0), p(r0, y0, a1), p(r1, y1, a1));
            b.tri(p(r0, y0, a0), p(r1, y1, a1), p(r1, y1, a0));
        }
    }
    for i in 0..segments {
        let a0 = (i as f32) / (segments as f32) * TAU;
        let a1 = ((i + 1) as f32) / (segments as f32) * TAU;
        let p0 = [a0.cos() * width * 0.5, 0.0, a0.sin() * depth * 0.5];
        let p1 = [a1.cos() * width * 0.5, 0.0, a1.sin() * depth * 0.5];
        b.tri([0.0, 0.0, 0.0], p1, p0);
    }
    b.mesh()
}

fn torus(width: f32, depth: f32, height: f32) -> Mesh {
    let mut b = Build::new();
    let outer = width.min(depth).max(0.4);
    let ratio = (height / outer).clamp(0.05, 0.49);
    let tube_r = ratio * outer * 0.5;
    let major = (outer * 0.5 - tube_r).max(0.2);
    let x_scale = width / (2.0 * (major + tube_r));
    let z_scale = depth / (2.0 * (major + tube_r));
    let y_scale = height / (tube_r * 2.0);
    let major_steps = 28;
    let tube_steps = 12;
    let point = |i: u32, j: u32| -> [f32; 3] {
        let theta = (i as f32) / (major_steps as f32) * TAU;
        let phi = (j as f32) / (tube_steps as f32) * TAU;
        let cx = (major + tube_r * phi.cos()) * theta.cos();
        let cz = (major + tube_r * phi.cos()) * theta.sin();
        let cy = tube_r * phi.sin() * y_scale + tube_r * y_scale;
        [cx * x_scale, cy, cz * z_scale]
    };
    for i in 0..major_steps {
        for j in 0..tube_steps {
            let a = point(i, j);
            let c = point(i + 1, j);
            let d = point(i + 1, j + 1);
            let e = point(i, j + 1);
            b.tri(a, c, d);
            b.tri(a, d, e);
        }
    }
    b.mesh()
}

fn tube(width: f32, depth: f32, height: f32, wall: f32) -> Mesh {
    let mut b = Build::new();
    let segments = CYLINDER_SEGMENTS;
    let limit = (width.min(depth) * 0.5 - 0.2).max(0.2);
    let wall = wall.clamp(0.2, limit);
    for i in 0..segments {
        let a0 = (i as f32) / (segments as f32) * TAU;
        let a1 = ((i + 1) as f32) / (segments as f32) * TAU;
        let ring = |radius_x: f32, radius_z: f32, y: f32, a: f32| {
            [a.cos() * radius_x, y, a.sin() * radius_z]
        };
        let ox = width * 0.5;
        let oz = depth * 0.5;
        let ix = (width * 0.5 - wall).max(0.1);
        let iz = (depth * 0.5 - wall).max(0.1);
        let o00 = ring(ox, oz, 0.0, a0);
        let o10 = ring(ox, oz, 0.0, a1);
        let o01 = ring(ox, oz, height, a0);
        let o11 = ring(ox, oz, height, a1);
        b.tri(o00, o10, o11);
        b.tri(o00, o11, o01);
        let i00 = ring(ix, iz, 0.0, a0);
        let i10 = ring(ix, iz, 0.0, a1);
        let i01 = ring(ix, iz, height, a0);
        let i11 = ring(ix, iz, height, a1);
        b.tri(i10, i00, i01);
        b.tri(i10, i01, i11);
        b.tri(o01, o11, i11);
        b.tri(o01, i11, i01);
        b.tri(o10, o00, i00);
        b.tri(o10, i00, i10);
    }
    b.mesh()
}

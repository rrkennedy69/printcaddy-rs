//! Mesh booleans. The clip is the BSP used by csg.js: each solid cuts the other's
//! triangles, and the kept pieces are stitched back into one mesh.

use glam::Vec3;

use crate::mesh::Mesh;
use crate::sketch::is_closed;

const EPS: f32 = 1.0e-3;
const MAX_TRIANGLES: usize = 12_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanError {
    NotClosed,
    Empty,
    Degenerate,
    TooComplex,
    TooFew,
}

impl std::fmt::Display for BooleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            BooleanError::NotClosed => "That body is not a closed solid.",
            BooleanError::Empty => "Those bodies do not overlap.",
            BooleanError::Degenerate => "The boolean did not produce a closed solid.",
            BooleanError::TooComplex => "That body has too many triangles to combine.",
            BooleanError::TooFew => "Select at least two bodies.",
        };
        f.write_str(text)
    }
}

pub fn mesh_volume(mesh: &Mesh) -> f32 {
    let mut volume = 0.0;
    for tri in mesh.indices.chunks_exact(3) {
        let a = Vec3::from_array(mesh.positions[tri[0] as usize]);
        let b = Vec3::from_array(mesh.positions[tri[1] as usize]);
        let c = Vec3::from_array(mesh.positions[tri[2] as usize]);
        volume += a.dot(b.cross(c));
    }
    volume / 6.0
}

pub fn boolean(op: BooleanOp, a: &Mesh, b: &Mesh) -> Result<Mesh, BooleanError> {
    if a.triangle_count() > MAX_TRIANGLES || b.triangle_count() > MAX_TRIANGLES {
        return Err(BooleanError::TooComplex);
    }
    if a.triangle_count() == 0 || b.triangle_count() == 0 || !is_closed(a) || !is_closed(b) {
        return Err(BooleanError::NotClosed);
    }
    if let Some(result) = convex_boolean(op, a, b) {
        return result;
    }
    let (a_poly, flip_a) = polygons_outward(a);
    let (b_poly, _) = polygons_outward(b);
    if a_poly.is_empty() || b_poly.is_empty() {
        return Err(BooleanError::Degenerate);
    }
    let mut tree_a = Node::from_polygons(a_poly);
    let mut tree_b = Node::from_polygons(b_poly);
    match op {
        BooleanOp::Union => {
            tree_a.clip_to(&tree_b);
            tree_b.clip_to(&tree_a);
            tree_b.invert();
            tree_b.clip_to(&tree_a);
            tree_b.invert();
            tree_a.build(tree_b.all_polygons());
        }
        BooleanOp::Subtract => {
            tree_a.invert();
            tree_a.clip_to(&tree_b);
            tree_b.clip_to(&tree_a);
            tree_b.invert();
            tree_b.clip_to(&tree_a);
            tree_b.invert();
            tree_a.build(tree_b.all_polygons());
            tree_a.invert();
        }
        BooleanOp::Intersect => {
            tree_a.invert();
            tree_b.clip_to(&tree_a);
            tree_b.invert();
            tree_a.clip_to(&tree_b);
            tree_b.clip_to(&tree_a);
            tree_a.build(tree_b.all_polygons());
            tree_a.invert();
        }
    }
    let mut mesh = mesh_from_polygons(&tree_a.all_polygons());
    snap_vertices(&mut mesh, 0.01);
    mesh = resolve_tjunctions(mesh);
    snap_vertices(&mut mesh, 0.01);
    mesh = dedup_triangles(mesh);
    if flip_a {
        reverse_winding(&mut mesh);
    }
    if mesh.triangle_count() == 0 || mesh_volume(&mesh).abs() < 1.0e-3 {
        return Err(BooleanError::Empty);
    }
    if !is_closed(&mesh) {
        return Err(BooleanError::Degenerate);
    }
    Ok(mesh)
}

fn convex_boolean(op: BooleanOp, a: &Mesh, b: &Mesh) -> Option<Result<Mesh, BooleanError>> {
    let (a_poly, flip_a) = polygons_outward(a);
    let (b_poly, _) = polygons_outward(b);
    if !convex_polygons(&a_poly) || !convex_polygons(&b_poly) {
        return None;
    }
    let a_planes: Vec<Plane> = a_poly.iter().map(|p| p.plane).collect();
    let b_planes: Vec<Plane> = b_poly.iter().map(|p| p.plane).collect();
    let a_out = clip_all(&a_poly, &b_planes, false);
    let b_out = clip_all(&b_poly, &a_planes, false);
    let a_in = clip_all(&a_poly, &b_planes, true);
    let b_in = clip_all(&b_poly, &a_planes, true);
    let polygons = match op {
        BooleanOp::Union => {
            let mut out = a_out;
            out.extend(b_out);
            out
        }
        BooleanOp::Intersect => {
            let mut out = a_in;
            out.extend(b_in);
            out
        }
        BooleanOp::Subtract => {
            let mut caps = b_in;
            for polygon in &mut caps {
                polygon.flip();
            }
            let mut out = a_out;
            out.extend(caps);
            out
        }
    };
    if polygons.is_empty() {
        return if op == BooleanOp::Intersect {
            Some(Err(BooleanError::Empty))
        } else {
            None
        };
    }
    let mut mesh = mesh_from_polygons(&polygons);
    snap_vertices(&mut mesh, 0.01);
    mesh = resolve_tjunctions(mesh);
    snap_vertices(&mut mesh, 0.01);
    mesh = dedup_triangles(mesh);
    if flip_a {
        reverse_winding(&mut mesh);
    }
    if mesh.triangle_count() == 0 || mesh_volume(&mesh).abs() < 1.0e-3 {
        return if op == BooleanOp::Intersect {
            Some(Err(BooleanError::Empty))
        } else {
            None
        };
    }
    if !is_closed(&mesh) {
        return None;
    }
    Some(Ok(mesh))
}

fn convex_polygons(polygons: &[Polygon]) -> bool {
    if polygons.is_empty() {
        return false;
    }
    let points: Vec<Vec3> = polygons
        .iter()
        .flat_map(|p| p.vertices.iter().copied())
        .collect();
    polygons.iter().all(|polygon| {
        points
            .iter()
            .all(|point| polygon.plane.normal.dot(*point) - polygon.plane.w <= 0.05)
    })
}

fn clip_all(polygons: &[Polygon], planes: &[Plane], keep_inside: bool) -> Vec<Polygon> {
    let mut out = Vec::new();
    for polygon in polygons {
        let mut current = Some(polygon.clone());
        for plane in planes {
            let Some(subject) = current else { break };
            current = clip_halfspace(subject, *plane, keep_inside);
        }
        if let Some(polygon) = current {
            out.push(polygon);
        }
    }
    out
}

fn clip_halfspace(polygon: Polygon, plane: Plane, keep_inside: bool) -> Option<Polygon> {
    let dist = |v: Vec3| plane.normal.dot(v) - plane.w;
    let inside = |d: f32| if keep_inside { d <= EPS } else { d >= -EPS };
    let mut output = Vec::new();
    let count = polygon.vertices.len();
    for i in 0..count {
        let start = polygon.vertices[i];
        let end = polygon.vertices[(i + 1) % count];
        let ds = dist(start);
        let de = dist(end);
        let start_in = inside(ds);
        let end_in = inside(de);
        if end_in {
            if !start_in {
                output.push(edge_hit(start, end, ds, de));
            }
            output.push(end);
        } else if start_in {
            output.push(edge_hit(start, end, ds, de));
        }
    }
    let vertices = clean(output)?;
    let plane = plane_of(&vertices)?;
    Some(Polygon { vertices, plane })
}

fn edge_hit(start: Vec3, end: Vec3, ds: f32, de: f32) -> Vec3 {
    let denom = ds - de;
    if denom.abs() < 1.0e-8 {
        return start;
    }
    let t = (ds / denom).clamp(0.0, 1.0);
    start + (end - start) * t
}

fn reverse_winding(mesh: &mut Mesh) {
    for tri in mesh.indices.chunks_exact_mut(3) {
        tri.swap(1, 2);
    }
    for normal in &mut mesh.normals {
        *normal = (-Vec3::from_array(*normal)).to_array();
    }
}

#[derive(Clone, Copy)]
struct Plane {
    normal: Vec3,
    w: f32,
}

impl Plane {
    fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Option<Self> {
        let normal = (b - a).cross(c - a);
        if normal.length_squared() < 1.0e-12 {
            return None;
        }
        let normal = normal.normalize();
        Some(Self {
            normal,
            w: normal.dot(a),
        })
    }

    fn flip(&mut self) {
        self.normal = -self.normal;
        self.w = -self.w;
    }
}

#[derive(Clone)]
struct Polygon {
    vertices: Vec<Vec3>,
    plane: Plane,
}

impl Polygon {
    fn flip(&mut self) {
        self.vertices.reverse();
        self.plane.flip();
    }
}

struct Node {
    plane: Option<Plane>,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
    polygons: Vec<Polygon>,
}

impl Node {
    fn from_polygons(polygons: Vec<Polygon>) -> Self {
        let mut node = Self {
            plane: None,
            front: None,
            back: None,
            polygons: Vec::new(),
        };
        node.build(polygons);
        node
    }

    fn build(&mut self, polygons: Vec<Polygon>) {
        if polygons.is_empty() {
            return;
        }
        if self.plane.is_none() {
            self.plane = Some(polygons[0].plane);
        }
        let plane = self.plane.unwrap();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for polygon in polygons {
            split_polygon(
                plane,
                polygon,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }
        self.polygons.extend(coplanar_front);
        self.polygons.extend(coplanar_back);
        if !front.is_empty() {
            self.front
                .get_or_insert_with(|| Box::new(Self::empty()))
                .build(front);
        }
        if !back.is_empty() {
            self.back
                .get_or_insert_with(|| Box::new(Self::empty()))
                .build(back);
        }
    }

    fn empty() -> Self {
        Self {
            plane: None,
            front: None,
            back: None,
            polygons: Vec::new(),
        }
    }

    fn invert(&mut self) {
        for polygon in &mut self.polygons {
            polygon.flip();
        }
        if let Some(plane) = &mut self.plane {
            plane.flip();
        }
        if let Some(front) = &mut self.front {
            front.invert();
        }
        if let Some(back) = &mut self.back {
            back.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    fn clip_polygons(&self, polygons: Vec<Polygon>) -> Vec<Polygon> {
        let Some(plane) = self.plane else {
            return polygons;
        };
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();
        let mut front = Vec::new();
        let mut back = Vec::new();
        for polygon in polygons {
            split_polygon(
                plane,
                polygon,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }
        front.extend(coplanar_front);
        back.extend(coplanar_back);
        let front = if let Some(node) = &self.front {
            node.clip_polygons(front)
        } else {
            front
        };
        let back = if let Some(node) = &self.back {
            node.clip_polygons(back)
        } else {
            Vec::new()
        };
        let mut out = front;
        out.extend(back);
        out
    }

    fn clip_to(&mut self, other: &Node) {
        self.polygons = other.clip_polygons(std::mem::take(&mut self.polygons));
        if let Some(front) = &mut self.front {
            front.clip_to(other);
        }
        if let Some(back) = &mut self.back {
            back.clip_to(other);
        }
    }

    fn all_polygons(&self) -> Vec<Polygon> {
        let mut out = self.polygons.clone();
        if let Some(front) = &self.front {
            out.extend(front.all_polygons());
        }
        if let Some(back) = &self.back {
            out.extend(back.all_polygons());
        }
        out
    }
}

fn split_polygon(
    plane: Plane,
    polygon: Polygon,
    coplanar_front: &mut Vec<Polygon>,
    coplanar_back: &mut Vec<Polygon>,
    front: &mut Vec<Polygon>,
    back: &mut Vec<Polygon>,
) {
    const COPLANAR: u8 = 0;
    const FRONT: u8 = 1;
    const BACK: u8 = 2;
    const SPANNING: u8 = 3;
    let mut kinds = Vec::with_capacity(polygon.vertices.len());
    let mut polygon_kind = 0u8;
    for vertex in &polygon.vertices {
        let t = plane.normal.dot(*vertex) - plane.w;
        let kind = if t < -EPS {
            BACK
        } else if t > EPS {
            FRONT
        } else {
            COPLANAR
        };
        polygon_kind |= kind;
        kinds.push(kind);
    }
    match polygon_kind {
        COPLANAR => {
            if plane.normal.dot(polygon.plane.normal) > 0.0 {
                coplanar_front.push(polygon);
            } else {
                coplanar_back.push(polygon);
            }
        }
        FRONT => front.push(polygon),
        BACK => back.push(polygon),
        _ => {
            let mut f = Vec::new();
            let mut b = Vec::new();
            let n = polygon.vertices.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let vi = polygon.vertices[i];
                let vj = polygon.vertices[j];
                let ti = kinds[i];
                let tj = kinds[j];
                if ti != BACK {
                    f.push(vi);
                }
                if ti != FRONT {
                    b.push(vi);
                }
                if (ti | tj) == SPANNING {
                    let denom = plane.normal.dot(vj - vi);
                    if denom.abs() > 1.0e-8 {
                        let t = ((plane.w - plane.normal.dot(vi)) / denom).clamp(0.0, 1.0);
                        let v = vi + (vj - vi) * t;
                        f.push(v);
                        b.push(v);
                    }
                }
            }
            if let Some(vertices) = clean(f) {
                if let Some(plane) = plane_of(&vertices) {
                    front.push(Polygon { vertices, plane });
                }
            }
            if let Some(vertices) = clean(b) {
                if let Some(plane) = plane_of(&vertices) {
                    back.push(Polygon { vertices, plane });
                }
            }
        }
    }
}

fn clean(vertices: Vec<Vec3>) -> Option<Vec<Vec3>> {
    let mut out = Vec::new();
    for vertex in vertices {
        if out
            .last()
            .is_some_and(|prev: &Vec3| (*prev - vertex).length_squared() < 1.0e-12)
        {
            continue;
        }
        out.push(vertex);
    }
    if out.len() >= 2 && (out[0] - *out.last().unwrap()).length_squared() < 1.0e-12 {
        out.pop();
    }
    (out.len() >= 3).then_some(out)
}

fn plane_of(vertices: &[Vec3]) -> Option<Plane> {
    Plane::from_points(vertices[0], vertices[1], vertices[2])
}

fn polygons_outward(mesh: &Mesh) -> (Vec<Polygon>, bool) {
    let flip = mesh_volume(mesh) < 0.0;
    let mut polygons = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let mut verts = [
            Vec3::from_array(mesh.positions[tri[0] as usize]),
            Vec3::from_array(mesh.positions[tri[1] as usize]),
            Vec3::from_array(mesh.positions[tri[2] as usize]),
        ];
        if flip {
            verts.swap(1, 2);
        }
        let Some(plane) = Plane::from_points(verts[0], verts[1], verts[2]) else {
            continue;
        };
        polygons.push(Polygon {
            vertices: verts.to_vec(),
            plane,
        });
    }
    (polygons, flip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::box_mesh;

    #[test]
    fn overlapping_box_union_has_the_combined_volume() {
        let a = box_mesh(20.0, 20.0, 20.0);
        let b = a.translated(Vec3::new(10.0, 0.0, 0.0));
        let mesh = boolean(BooleanOp::Union, &a, &b).unwrap();
        let volume = mesh_volume(&mesh).abs();
        assert!((volume - 12_000.0).abs() < 50.0, "volume {volume}");
    }

    #[test]
    fn subtract_and_intersect_split_the_overlap() {
        let a = box_mesh(20.0, 20.0, 20.0);
        let b = a.translated(Vec3::new(10.0, 0.0, 0.0));
        let cut = boolean(BooleanOp::Subtract, &a, &b).unwrap();
        let shared = boolean(BooleanOp::Intersect, &a, &b).unwrap();
        assert!((mesh_volume(&cut).abs() - 4_000.0).abs() < 50.0);
        assert!((mesh_volume(&shared).abs() - 4_000.0).abs() < 50.0);
        assert!(is_closed(&cut) && is_closed(&shared));
    }

    #[test]
    fn disjoint_boxes_do_not_intersect() {
        let a = box_mesh(20.0, 20.0, 20.0);
        let b = a.translated(Vec3::new(80.0, 0.0, 0.0));
        assert_eq!(
            boolean(BooleanOp::Intersect, &a, &b).unwrap_err(),
            BooleanError::Empty
        );
        let joined = boolean(BooleanOp::Union, &a, &b).unwrap();
        assert!(is_closed(&joined));
        assert!((mesh_volume(&joined).abs() - 16_000.0).abs() < 50.0);
    }

    #[test]
    fn a_cylinder_cuts_a_box() {
        let stock = box_mesh(20.0, 20.0, 20.0);
        let cutter = crate::mesh::cylinder_mesh(10.0, 20.0, 24);
        let mesh = boolean(BooleanOp::Subtract, &stock, &cutter).unwrap();
        assert!(is_closed(&mesh));
        let volume = mesh_volume(&mesh).abs();
        assert!(volume < 7_000.0 && volume > 1_000.0, "volume {volume}");
    }
}

fn dedup_triangles(mesh: Mesh) -> Mesh {
    use std::collections::HashMap;
    let key = |p: [f32; 3]| {
        [
            (p[0] * 1000.0).round() as i32,
            (p[1] * 1000.0).round() as i32,
            (p[2] * 1000.0).round() as i32,
        ]
    };
    let mut groups: HashMap<[[i32; 3]; 3], Vec<[u32; 3]>> = HashMap::new();
    for tri in mesh.indices.chunks_exact(3) {
        let tri = [tri[0], tri[1], tri[2]];
        let mut verts = [
            key(mesh.positions[tri[0] as usize]),
            key(mesh.positions[tri[1] as usize]),
            key(mesh.positions[tri[2] as usize]),
        ];
        if verts[0] == verts[1] || verts[1] == verts[2] || verts[0] == verts[2] {
            continue;
        }
        let flipped = if verts[0] > verts[1] {
            verts.swap(0, 1);
            true
        } else {
            false
        };
        let flipped = if verts[1] > verts[2] {
            verts.swap(1, 2);
            !flipped
        } else {
            flipped
        };
        let flipped = if verts[0] > verts[1] {
            verts.swap(0, 1);
            !flipped
        } else {
            flipped
        };
        let _ = flipped;
        groups.entry(verts).or_default().push(tri);
    }
    let mut indices = Vec::new();
    for tris in groups.into_values() {
        indices.extend_from_slice(&tris[0]);
    }
    Mesh {
        positions: mesh.positions,
        normals: mesh.normals,
        indices,
    }
}

fn snap_vertices(mesh: &mut Mesh, step: f32) {
    for position in &mut mesh.positions {
        for coord in position.iter_mut() {
            *coord = (*coord / step).round() * step;
        }
    }
}

fn resolve_tjunctions(mesh: Mesh) -> Mesh {
    let mut unique = Vec::new();
    for position in &mesh.positions {
        let point = Vec3::from_array(*position);
        if unique
            .iter()
            .all(|kept: &Vec3| (*kept - point).length_squared() > 1.0e-8)
        {
            unique.push(point);
        }
    }
    let mut triangles: Vec<[Vec3; 3]> = mesh
        .indices
        .chunks_exact(3)
        .map(|tri| {
            [
                Vec3::from_array(mesh.positions[tri[0] as usize]),
                Vec3::from_array(mesh.positions[tri[1] as usize]),
                Vec3::from_array(mesh.positions[tri[2] as usize]),
            ]
        })
        .collect();
    for _ in 0..triangles.len().saturating_mul(4).max(8) {
        let mut next = Vec::with_capacity(triangles.len());
        let mut split_any = false;
        for tri in triangles {
            if let Some((edge, point)) = junction(&tri, &unique) {
                split_any = true;
                let (a, b) = split_edge(tri, edge, point);
                next.push(a);
                next.push(b);
            } else {
                next.push(tri);
            }
        }
        triangles = next;
        if !split_any {
            break;
        }
    }
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for tri in triangles {
        let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        if normal.length_squared() < 1.0e-10 {
            continue;
        }
        let normal = normal.normalize();
        let base = positions.len() as u32;
        for point in tri {
            positions.push(point.to_array());
            normals.push(normal.to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Mesh {
        positions,
        normals,
        indices,
    }
}

fn junction(tri: &[Vec3; 3], points: &[Vec3]) -> Option<(usize, Vec3)> {
    for edge in 0..3 {
        let a = tri[edge];
        let b = tri[(edge + 1) % 3];
        for point in points {
            if on_segment(*point, a, b) {
                return Some((edge, *point));
            }
        }
    }
    None
}

fn on_segment(point: Vec3, a: Vec3, b: Vec3) -> bool {
    if (point - a).length_squared() < 1.0e-8 || (point - b).length_squared() < 1.0e-8 {
        return false;
    }
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1.0e-10 {
        return false;
    }
    let t = (point - a).dot(ab) / len2;
    if !(0.002..=0.998).contains(&t) {
        return false;
    }
    (a + ab * t - point).length() < 0.05
}

fn split_edge(tri: [Vec3; 3], edge: usize, point: Vec3) -> ([Vec3; 3], [Vec3; 3]) {
    let a = tri[edge];
    let b = tri[(edge + 1) % 3];
    let c = tri[(edge + 2) % 3];
    ([a, point, c], [point, b, c])
}

fn mesh_from_polygons(polygons: &[Polygon]) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for polygon in polygons {
        if polygon.vertices.len() < 3 {
            continue;
        }
        let normal = polygon.plane.normal;
        let base = positions.len() as u32;
        for vertex in &polygon.vertices {
            positions.push(vertex.to_array());
            normals.push(normal.to_array());
        }
        for i in 1..polygon.vertices.len() - 1 {
            indices.extend_from_slice(&[base, base + i as u32, base + i as u32 + 1]);
        }
    }
    Mesh {
        positions,
        normals,
        indices,
    }
}

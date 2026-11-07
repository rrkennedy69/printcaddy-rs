use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }
}

/// Intersection with the horizontal plane `y = plane_y`. `None` when the ray is parallel
/// or the plane is behind the origin.
pub fn ray_plane_y(ray: Ray, plane_y: f32) -> Option<Vec3> {
    if ray.direction.y.abs() < 1e-6 {
        return None;
    }
    let t = (plane_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + ray.direction * t)
}

pub(crate) fn ray_aabb(ray: Ray, min: Vec3, max: Vec3) -> Option<f32> {
    let inv = Vec3::new(
        if ray.direction.x.abs() < 1e-8 {
            f32::INFINITY
        } else {
            1.0 / ray.direction.x
        },
        if ray.direction.y.abs() < 1e-8 {
            f32::INFINITY
        } else {
            1.0 / ray.direction.y
        },
        if ray.direction.z.abs() < 1e-8 {
            f32::INFINITY
        } else {
            1.0 / ray.direction.z
        },
    );
    let t1 = (min - ray.origin) * inv;
    let t2 = (max - ray.origin) * inv;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let enter = tmin.x.max(tmin.y).max(tmin.z);
    let exit = tmax.x.min(tmax.y).min(tmax.z);
    if exit >= enter.max(0.0) {
        Some(enter.max(0.0))
    } else {
        None
    }
}

/// Finite cylinder along Y, radius `radius`, from y=0 to y=`height`, in local space.
pub(crate) fn ray_cylinder(ray: Ray, radius: f32, height: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    let consider = |t: f32, best: &mut Option<f32>| {
        if t >= 0.0 && best.map(|b| t < b).unwrap_or(true) {
            *best = Some(t);
        }
    };

    // Side wall: x^2 + z^2 = r^2.
    let (ox, oz) = (ray.origin.x, ray.origin.z);
    let (dx, dz) = (ray.direction.x, ray.direction.z);
    let a = dx * dx + dz * dz;
    let b = 2.0 * (ox * dx + oz * dz);
    let c = ox * ox + oz * oz - radius * radius;
    if a > 1e-8 {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let root = disc.sqrt();
            for sign in [-1.0, 1.0] {
                let t = (-b + sign * root) / (2.0 * a);
                if t >= 0.0 {
                    let y = ray.origin.y + ray.direction.y * t;
                    if (0.0..=height).contains(&y) {
                        consider(t, &mut best);
                    }
                }
            }
        }
    }

    if ray.direction.y.abs() > 1e-8 {
        for y in [0.0, height] {
            let t = (y - ray.origin.y) / ray.direction.y;
            if t >= 0.0 {
                let p = ray.origin + ray.direction * t;
                if p.x * p.x + p.z * p.z <= radius * radius + 1e-4 {
                    consider(t, &mut best);
                }
            }
        }
    }
    best
}

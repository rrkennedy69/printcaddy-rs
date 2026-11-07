use glam::{Mat4, Vec3};

/// Orbit camera looking at a point on the build plate. Y is up.
#[derive(Clone, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.85,
            pitch: 0.72,
            distance: 420.0,
        }
    }
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let pitch = self.pitch.clamp(-1.35, 1.35);
        let cp = pitch.cos();
        self.target
            + Vec3::new(self.yaw.sin() * cp, pitch.sin(), self.yaw.cos() * cp) * self.distance
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let aspect = if aspect.is_finite() && aspect > 0.05 {
            aspect
        } else {
            1.0
        };
        Mat4::perspective_rh(50.0_f32.to_radians(), aspect, 0.5, 8000.0)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.35, 1.35);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let eye = self.eye();
        let forward = (self.target - eye).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let scale = self.distance * 0.0015;
        self.target += (-right * dx + up * dy) * scale;
    }

    pub fn zoom_factor(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.distance = (self.distance * factor).clamp(40.0, 2400.0);
        }
    }
}

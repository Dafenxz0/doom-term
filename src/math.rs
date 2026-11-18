#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn scale(self, factor: f32) -> Self {
        let f = factor.max(0.0);
        Self {
            r: (f32::from(self.r) * f).clamp(0.0, 255.0) as u8,
            g: (f32::from(self.g) * f).clamp(0.0, 255.0) as u8,
            b: (f32::from(self.b) * f).clamp(0.0, 255.0) as u8,
        }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self {
            r: (f32::from(self.r) * inv + f32::from(other.r) * t).clamp(0.0, 255.0) as u8,
            g: (f32::from(self.g) * inv + f32::from(other.g) * t).clamp(0.0, 255.0) as u8,
            b: (f32::from(self.b) * inv + f32::from(other.b) * t).clamp(0.0, 255.0) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn rotate(self, angle_rad: f32) -> Self {
        let (sin_a, cos_a) = angle_rad.sin_cos();
        Self {
            x: self.x * cos_a - self.y * sin_a,
            y: self.x * sin_a + self.y * cos_a,
        }
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pos: Vec2,
    pub dir: Vec2,
    pub plane: Vec2,
}

impl Camera {
    pub fn from_angle(pos: Vec2, angle_rad: f32, fov_scale: f32) -> Self {
        let dir = Vec2::new(angle_rad.cos(), angle_rad.sin());
        let plane = Vec2::new(-dir.y * fov_scale, dir.x * fov_scale);
        Self { pos, dir, plane }
    }

    pub fn rotate(&mut self, delta_rad: f32) {
        self.dir = self.dir.rotate(delta_rad);
        self.plane = self.plane.rotate(delta_rad);
    }

    pub fn angle_deg(&self) -> f32 {
        let deg = self.dir.y.atan2(self.dir.x).to_degrees();
        if deg < 0.0 {
            deg + 360.0
        } else {
            deg
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LightingConfig {
    pub flashlight_on: bool,
    pub fog_color: Rgb,
    pub ambient: f32,
    pub fog_density: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            flashlight_on: true,
            fog_color: Rgb::new(8, 10, 15),
            ambient: 0.72,
            fog_density: 0.14,
        }
    }
}

pub fn apply_lighting(
    base: Rgb,
    dist: f32,
    screen_x_norm: f32,
    screen_y_norm: f32,
    side_shade: f32,
    lighting: &LightingConfig,
) -> Rgb {
    let d = dist.max(0.05);
    let fog = (-d * lighting.fog_density).exp().clamp(0.0, 1.0);

    let (light_factor, fog_mix) = if lighting.flashlight_on {
        let radial = (screen_x_norm * screen_x_norm + 0.55 * screen_y_norm * screen_y_norm).sqrt();
        let cone = (1.0 - radial / 0.75).clamp(0.0, 1.0);
        let cone_smooth = cone * cone * (3.0 - 2.0 * cone);
        let beam_falloff = (1.0 / (1.0 + 0.04 * d * d)).clamp(0.0, 1.0);
        let boost = 0.85 * cone_smooth * beam_falloff;
        let lf = (lighting.ambient * fog + boost) * side_shade;
        let fm = (1.0 - fog) * (1.0 - 0.65 * cone_smooth * beam_falloff);
        (lf.clamp(0.05, 1.35), fm.clamp(0.0, 1.0))
    } else {
        let dim_fog = (-d * (lighting.fog_density * 1.45)).exp().clamp(0.0, 1.0);
        let lf = lighting.ambient * 0.52 * dim_fog * side_shade;
        (lf.clamp(0.04, 1.0), (1.0 - dim_fog).clamp(0.0, 1.0))
    };

    base.scale(light_factor).lerp(lighting.fog_color, fog_mix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flashlight_brightens_center_ray_more_than_edge() {
        let cfg = LightingConfig::default();
        let base = Rgb::new(200, 200, 200);
        let center = apply_lighting(base, 4.0, 0.0, 0.0, 1.0, &cfg);
        let edge = apply_lighting(base, 4.0, 0.95, 0.0, 1.0, &cfg);
        assert!(center.r > edge.r);
    }

    #[test]
    fn distant_geometry_fades_toward_fog() {
        let cfg = LightingConfig {
            flashlight_on: false,
            ..LightingConfig::default()
        };
        let base = Rgb::new(220, 60, 40);
        let near = apply_lighting(base, 1.0, 0.0, 0.0, 1.0, &cfg);
        let far = apply_lighting(base, 25.0, 0.0, 0.0, 1.0, &cfg);
        assert!(near.r > far.r);
        assert!((i16::from(far.r) - i16::from(cfg.fog_color.r)).abs() <= 15);
    }
}

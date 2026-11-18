use crate::math::Rgb;

pub const TEX_SIZE: usize = 64;
const TEX_LEN: usize = TEX_SIZE * TEX_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallKind {
    Brick,
    TechMetal,
    Hazard,
    Stone,
    Door,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpriteKind {
    EnergyOrb,
    Pillar,
    CyberDrone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Texel {
    pub rgb: Rgb,
    pub opaque: bool,
}

impl Texel {
    const TRANSPARENT: Self = Self {
        rgb: Rgb::BLACK,
        opaque: false,
    };

    const fn solid(r: u8, g: u8, b: u8) -> Self {
        Self {
            rgb: Rgb::new(r, g, b),
            opaque: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextureAtlas {
    brick: Vec<Rgb>,
    tech_metal: Vec<Rgb>,
    hazard: Vec<Rgb>,
    stone: Vec<Rgb>,
    door: Vec<Rgb>,
    floor: Vec<Rgb>,
    ceiling: Vec<Rgb>,
    orb: Vec<Texel>,
    pillar: Vec<Texel>,
    drone: Vec<Texel>,
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureAtlas {
    pub fn new() -> Self {
        Self {
            brick: gen_brick(),
            tech_metal: gen_tech_metal(),
            hazard: gen_hazard(),
            stone: gen_stone(),
            door: gen_door(),
            floor: gen_floor(),
            ceiling: gen_ceiling(),
            orb: gen_orb(),
            pillar: gen_pillar(),
            drone: gen_drone(),
        }
    }

    pub fn wall_texture(&self, kind: WallKind) -> &[Rgb] {
        match kind {
            WallKind::Brick => &self.brick,
            WallKind::TechMetal => &self.tech_metal,
            WallKind::Hazard => &self.hazard,
            WallKind::Stone => &self.stone,
            WallKind::Door => &self.door,
        }
    }

    pub fn sample_wall(&self, kind: WallKind, u: f32, v: f32) -> Rgb {
        let tex = self.wall_texture(kind);
        let tx = ((u.rem_euclid(1.0)) * TEX_SIZE as f32) as usize % TEX_SIZE;
        let ty = ((v.clamp(0.0, 0.999_99)) * TEX_SIZE as f32) as usize % TEX_SIZE;
        tex[ty * TEX_SIZE + tx]
    }

    pub fn sample_floor(&self, world_x: f32, world_y: f32) -> Rgb {
        let tx = (world_x.rem_euclid(1.0) * TEX_SIZE as f32) as usize % TEX_SIZE;
        let ty = (world_y.rem_euclid(1.0) * TEX_SIZE as f32) as usize % TEX_SIZE;
        self.floor[ty * TEX_SIZE + tx]
    }

    pub fn sample_ceiling(&self, world_x: f32, world_y: f32) -> Rgb {
        let tx = (world_x.rem_euclid(1.0) * TEX_SIZE as f32) as usize % TEX_SIZE;
        let ty = (world_y.rem_euclid(1.0) * TEX_SIZE as f32) as usize % TEX_SIZE;
        self.ceiling[ty * TEX_SIZE + tx]
    }

    pub fn sample_sprite(&self, kind: SpriteKind, tx: usize, ty: usize) -> Texel {
        let x = tx.min(TEX_SIZE - 1);
        let y = ty.min(TEX_SIZE - 1);
        let idx = y * TEX_SIZE + x;
        match kind {
            SpriteKind::EnergyOrb => self.orb[idx],
            SpriteKind::Pillar => self.pillar[idx],
            SpriteKind::CyberDrone => self.drone[idx],
        }
    }
}

fn hash2d(x: usize, y: usize, seed: u32) -> i16 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(1_274_126_177);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0x1f) as i16 - 16
}

fn add_noise(base: Rgb, delta: i16) -> Rgb {
    Rgb::new(
        (i16::from(base.r) + delta).clamp(0, 255) as u8,
        (i16::from(base.g) + delta).clamp(0, 255) as u8,
        (i16::from(base.b) + delta).clamp(0, 255) as u8,
    )
}

fn gen_brick() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        let row = y / 8;
        let offset = if row % 2 == 1 { 8 } else { 0 };
        for x in 0..TEX_SIZE {
            let bx = (x + offset) % 16;
            let by = y % 8;
            let is_mortar = by == 0 || bx == 0;
            let n = hash2d(x, y, 1) / 2;
            let color = if is_mortar {
                add_noise(Rgb::new(58, 54, 52), n / 2)
            } else {
                let block_tint = hash2d((x + offset) / 16, row, 9) / 2;
                let base = Rgb::new(
                    (148 + block_tint).clamp(90, 200) as u8,
                    (64 + block_tint / 2).clamp(36, 100) as u8,
                    (46 + block_tint / 3).clamp(24, 80) as u8,
                );
                let top_bevel = if by == 1 { 10 } else { 0 };
                add_noise(base, n + top_bevel)
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_tech_metal() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let border = x == 0 || x == 63 || y == 0 || y == 63 || x == 31 || x == 32;
            let rivet = (x == 4 || x == 27 || x == 36 || x == 59)
                && (y == 4 || y == 26 || y == 37 || y == 59);
            let glow_strip = (y == 31 || y == 32) && (6..58).contains(&x) && x != 31 && x != 32;
            let rib = y.is_multiple_of(8) && (8..56).contains(&x) && !(28..=35).contains(&x);
            let n = hash2d(x, y, 2) / 3;
            let color = if glow_strip {
                Rgb::new(50, 225, 255)
            } else if rivet {
                Rgb::new(175, 190, 205)
            } else if border {
                Rgb::new(42, 50, 62)
            } else if rib {
                Rgb::new(66, 78, 94)
            } else {
                add_noise(Rgb::new(88, 102, 120), n)
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_hazard() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let trim = !(8..56).contains(&y);
            let n = hash2d(x, y, 3) / 3;
            let color = if trim {
                let rivet = y == 4 || y == 59;
                if rivet && x % 12 == 6 {
                    Rgb::new(150, 155, 165)
                } else {
                    add_noise(Rgb::new(52, 56, 64), n)
                }
            } else if y == 8 || y == 55 {
                Rgb::new(28, 28, 30)
            } else {
                let stripe = ((x + y) / 8).is_multiple_of(2);
                if stripe {
                    add_noise(Rgb::new(225, 178, 24), n)
                } else {
                    add_noise(Rgb::new(30, 32, 36), n)
                }
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_stone() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        let row = y / 16;
        let offset = if row % 2 == 1 { 16 } else { 0 };
        for x in 0..TEX_SIZE {
            let sx = (x + offset) % 32;
            let sy = y % 16;
            let seam = sx == 0 || sy == 0;
            let n = hash2d(x, y, 4);
            let block_var = hash2d((x + offset) / 32, row, 17) / 2;
            let moss = (y > 46 || x % 29 == 3) && n > 9;
            let color = if seam {
                Rgb::new(38, 40, 44)
            } else if moss {
                add_noise(Rgb::new(62, 92, 58), n / 2)
            } else {
                add_noise(
                    Rgb::new(
                        (96 + block_var).clamp(55, 140) as u8,
                        (98 + block_var).clamp(55, 142) as u8,
                        (104 + block_var).clamp(60, 148) as u8,
                    ),
                    n / 2,
                )
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_door() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let frame = x < 4 || x >= 60 || y < 4 || y >= 60;
            let center_seam = x == 31 || x == 32;
            let badge = (22..42).contains(&x) && (24..40).contains(&y);
            let stripe = badge && ((x + y) / 4).is_multiple_of(2);
            let n = hash2d(x, y, 5) / 3;
            let color = if frame {
                Rgb::new(45, 52, 62)
            } else if center_seam {
                Rgb::new(22, 26, 30)
            } else if badge {
                if x == 22 || x == 41 || y == 24 || y == 39 {
                    Rgb::new(25, 28, 32)
                } else if stripe {
                    Rgb::new(220, 120, 30)
                } else {
                    Rgb::new(36, 38, 42)
                }
            } else if y.is_multiple_of(12) {
                Rgb::new(72, 82, 96)
            } else {
                add_noise(Rgb::new(108, 122, 138), n)
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_floor() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let grout = x.is_multiple_of(32) || y.is_multiple_of(32);
            let diamond =
                (x % 8 == 4 && y % 8 == 4) || (x.is_multiple_of(8) && y.is_multiple_of(8));
            let n = hash2d(x, y, 6) / 3;
            let color = if grout {
                Rgb::new(26, 30, 36)
            } else if diamond {
                Rgb::new(68, 74, 84)
            } else {
                add_noise(Rgb::new(48, 52, 60), n)
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_ceiling() -> Vec<Rgb> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let beam = x < 3 || x >= 61 || y < 3 || y >= 61;
            let vent = (14..50).contains(&x) && (14..50).contains(&y) && y.is_multiple_of(4);
            let n = hash2d(x, y, 7) / 4;
            let color = if beam {
                Rgb::new(28, 32, 42)
            } else if vent {
                Rgb::new(20, 24, 30)
            } else {
                add_noise(Rgb::new(38, 44, 56), n)
            };
            buf.push(color);
        }
    }
    buf
}

fn gen_orb() -> Vec<Texel> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let dx = x as f32 - 31.5;
            let dy = y as f32 - 31.5;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 23.0 {
                buf.push(Texel::TRANSPARENT);
            } else {
                let t = 1.0 - (dist / 23.0);
                let core = (t * t).clamp(0.0, 1.0);
                let r = (70.0 + 185.0 * core).clamp(0.0, 255.0) as u8;
                let g = (190.0 + 65.0 * core).clamp(0.0, 255.0) as u8;
                let b = 255;
                buf.push(Texel::solid(r, g, b));
            }
        }
    }
    buf
}

fn gen_pillar() -> Vec<Texel> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let cap = !(8..56).contains(&y);
            let half_w = if cap { 20 } else { 14 };
            let dx = (x as i32 - 32).abs();
            if dx > half_w {
                buf.push(Texel::TRANSPARENT);
            } else {
                let norm = dx as f32 / half_w as f32;
                let cyl = (1.0 - norm * norm).sqrt();
                let band = y == 8 || y == 55 || y == 31 || y == 32;
                let base = if band {
                    Rgb::new(180, 145, 70)
                } else if cap {
                    Rgb::new(115, 122, 134)
                } else {
                    Rgb::new(142, 148, 160)
                };
                let lit = base.scale(0.35 + 0.75 * cyl);
                buf.push(Texel {
                    rgb: lit,
                    opaque: true,
                });
            }
        }
    }
    buf
}

fn gen_drone() -> Vec<Texel> {
    let mut buf = Vec::with_capacity(TEX_LEN);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let dx = x as i32 - 32;
            let dy = y as i32 - 28;
            let body = dx * dx + dy * dy <= 13 * 13;
            let wing = dy.abs() <= 4 && dx.abs() <= 24 && dx.abs() >= 11;
            let antenna = dx.abs() <= 1 && (8..16).contains(&y);
            let thruster = dx.abs() <= 5 && (42..50).contains(&y);
            let visor = dy.abs() <= 2 && dx.abs() <= 8;

            let texel = if visor {
                if dx.abs() <= 2 {
                    Texel::solid(255, 70, 50)
                } else {
                    Texel::solid(220, 40, 30)
                }
            } else if body {
                let shade = 1.0 - ((dx * dx + dy * dy) as f32 / 220.0).min(0.6);
                let col = Rgb::new(118, 128, 144).scale(shade);
                Texel {
                    rgb: col,
                    opaque: true,
                }
            } else if wing {
                Texel::solid(74, 84, 98)
            } else if antenna {
                if y <= 10 {
                    Texel::solid(255, 190, 40)
                } else {
                    Texel::solid(140, 150, 165)
                }
            } else if thruster {
                if y >= 46 {
                    Texel::solid(60, 210, 255)
                } else {
                    Texel::solid(62, 68, 78)
                }
            } else {
                Texel::TRANSPARENT
            };
            buf.push(texel);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_procedural_textures_have_exact_64x64_bounds() {
        let atlas = TextureAtlas::new();
        for kind in [
            WallKind::Brick,
            WallKind::TechMetal,
            WallKind::Hazard,
            WallKind::Stone,
            WallKind::Door,
        ] {
            let tex = atlas.wall_texture(kind);
            assert_eq!(tex.len(), TEX_SIZE * TEX_SIZE);
            assert!(tex.windows(2).any(|w| w[0] != w[1]));
        }
    }

    #[test]
    fn sprite_textures_contain_transparent_and_opaque_texels() {
        let atlas = TextureAtlas::new();
        for kind in [
            SpriteKind::EnergyOrb,
            SpriteKind::Pillar,
            SpriteKind::CyberDrone,
        ] {
            let mut opaque_count = 0usize;
            let mut transparent_count = 0usize;
            for y in 0..TEX_SIZE {
                for x in 0..TEX_SIZE {
                    if atlas.sample_sprite(kind, x, y).opaque {
                        opaque_count += 1;
                    } else {
                        transparent_count += 1;
                    }
                }
            }
            assert!(opaque_count > 100);
            assert!(transparent_count > 100);
        }
    }

    #[test]
    fn sampling_out_of_range_uv_never_panics() {
        let atlas = TextureAtlas::new();
        let _ = atlas.sample_wall(WallKind::Hazard, -3.75, 4.25);
        let _ = atlas.sample_floor(-10.3, 99.8);
        let _ = atlas.sample_ceiling(0.0, -0.001);
        let _ = atlas.sample_sprite(SpriteKind::CyberDrone, 999, 999);
    }
}

use crate::map::{Map, SpriteSpawn};
use crate::math::{apply_lighting, Camera, LightingConfig, Rgb, Vec2};
use crate::raycaster::Framebuffer;
use crate::textures::{SpriteKind, TextureAtlas, TEX_SIZE};

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub pos: Vec2,
    pub kind: SpriteKind,
    pub phase: f32,
    pub patrol_dir: Vec2,
}

impl Sprite {
    pub fn from_spawn(spawn: SpriteSpawn, index: usize) -> Self {
        let patrol_dir = if index.is_multiple_of(2) {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(0.0, 1.0)
        };
        Self {
            pos: Vec2::new(spawn.x, spawn.y),
            kind: spawn.kind,
            phase: index as f32 * 1.17,
            patrol_dir,
        }
    }
}

pub fn update_sprites(sprites: &mut [Sprite], map: &Map, dt: f32) {
    const DRONE_SPEED: f32 = 0.85;
    const CLEARANCE: f32 = 0.35;

    for sprite in sprites.iter_mut() {
        if sprite.kind != SpriteKind::CyberDrone {
            continue;
        }
        let step = Vec2::new(
            sprite.patrol_dir.x * DRONE_SPEED * dt,
            sprite.patrol_dir.y * DRONE_SPEED * dt,
        );
        let probe_x = sprite.pos.x + step.x + sprite.patrol_dir.x * CLEARANCE;
        let probe_y = sprite.pos.y + step.y + sprite.patrol_dir.y * CLEARANCE;
        if map.is_blocking(probe_x, probe_y) {
            sprite.patrol_dir = Vec2::new(-sprite.patrol_dir.x, -sprite.patrol_dir.y);
        } else {
            sprite.pos.x += step.x;
            sprite.pos.y += step.y;
        }
    }
}

pub fn render_sprites(
    fb: &mut Framebuffer,
    camera: &Camera,
    sprites: &[Sprite],
    atlas: &TextureAtlas,
    lighting: &LightingConfig,
    time: f32,
) {
    if fb.width == 0 || fb.height == 0 || sprites.is_empty() {
        return;
    }

    let det = camera.plane.x * camera.dir.y - camera.dir.x * camera.plane.y;
    if det.abs() < 1e-8 {
        return;
    }
    let inv_det = 1.0 / det;

    let mut order: Vec<(usize, f32)> = sprites
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let d = Vec2::new(s.pos.x - camera.pos.x, s.pos.y - camera.pos.y);
            (i, d.length_sq())
        })
        .collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1));

    let w_i32 = fb.width as i32;
    let h_i32 = fb.height as i32;
    let half_h = fb.height as f32 * 0.5;

    for (idx, _) in order {
        let sprite = &sprites[idx];
        let dx = sprite.pos.x - camera.pos.x;
        let dy = sprite.pos.y - camera.pos.y;

        let transform_x = inv_det * (camera.dir.y * dx - camera.dir.x * dy);
        let transform_y = inv_det * (-camera.plane.y * dx + camera.plane.x * dy);
        if transform_y <= 0.15 {
            continue;
        }

        let sprite_screen_x = ((fb.width as f32 * 0.5) * (1.0 + transform_x / transform_y)) as i32;

        let (scale, bob_amp, bob_freq) = match sprite.kind {
            SpriteKind::Pillar => (1.0, 0.0, 0.0),
            SpriteKind::EnergyOrb => (0.55, 0.12, 3.4),
            SpriteKind::CyberDrone => (0.72, 0.08, 4.2),
        };
        let bob = if bob_amp > 0.0 {
            (time * bob_freq + sprite.phase).sin() * bob_amp
        } else {
            0.0
        };
        let v_move = (bob * fb.height as f32 / transform_y) as i32;

        let sprite_h = ((fb.height as f32 / transform_y) * scale).abs() as i32;
        if sprite_h <= 0 {
            continue;
        }
        let sprite_w = sprite_h;

        let raw_start_y = -sprite_h / 2 + h_i32 / 2 + v_move;
        let raw_end_y = sprite_h / 2 + h_i32 / 2 + v_move;
        let draw_start_y = raw_start_y.max(0);
        let draw_end_y = raw_end_y.min(h_i32);

        let raw_start_x = -sprite_w / 2 + sprite_screen_x;
        let raw_end_x = sprite_w / 2 + sprite_screen_x;
        let draw_start_x = raw_start_x.max(0);
        let draw_end_x = raw_end_x.min(w_i32);

        for stripe in draw_start_x..draw_end_x {
            let ux = stripe as usize;
            if transform_y >= fb.z_buffer[ux] {
                continue;
            }
            let tex_x = (((stripe - raw_start_x) * TEX_SIZE as i32) / sprite_w)
                .clamp(0, TEX_SIZE as i32 - 1) as usize;
            let screen_x_norm = 2.0 * (stripe as f32 + 0.5) / fb.width as f32 - 1.0;

            for y in draw_start_y..draw_end_y {
                let tex_y = (((y - raw_start_y) * TEX_SIZE as i32) / sprite_h)
                    .clamp(0, TEX_SIZE as i32 - 1) as usize;
                let texel = atlas.sample_sprite(sprite.kind, tex_x, tex_y);
                if !texel.opaque {
                    continue;
                }
                let screen_y_norm = (y as f32 - half_h) / half_h;
                let base = if sprite.kind == SpriteKind::EnergyOrb {
                    let pulse = 0.85 + 0.25 * (time * 5.0 + sprite.phase).sin();
                    texel.rgb.scale(pulse)
                } else {
                    texel.rgb
                };

                let mut shaded = apply_lighting(
                    base,
                    transform_y,
                    screen_x_norm,
                    screen_y_norm,
                    1.0,
                    lighting,
                );
                if sprite.kind == SpriteKind::EnergyOrb {
                    let glow = (-transform_y * 0.08).exp().clamp(0.15, 1.0);
                    shaded = Rgb::new(
                        shaded.r.max((f32::from(base.r) * glow * 0.7) as u8),
                        shaded.g.max((f32::from(base.g) * glow * 0.7) as u8),
                        shaded.b.max((f32::from(base.b) * glow * 0.7) as u8),
                    );
                }
                fb.set_pixel(ux, y as usize, shaded);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raycaster::render_world;

    #[test]
    fn z_buffer_occludes_sprite_behind_wall_and_draws_sprite_in_front() {
        // Wall at x=3, orb behind wall at x=4.5, orb in front at x=2.2
        let map = Map::parse("ztest", "######\n#P...#\n######", 0.0).unwrap();
        let mut wall_map = map.clone();
        // Place a Brick wall directly at (3, 1)
        wall_map.tiles[wall_map.width + 3] =
            crate::map::Tile::Wall(crate::textures::WallKind::Brick);

        let atlas = TextureAtlas::new();
        let lighting = LightingConfig::default();
        let camera = Camera::from_angle(wall_map.spawn_pos, 0.0, 0.66);

        let mut fb_no_sprite = Framebuffer::new(64, 48);
        render_world(&mut fb_no_sprite, &wall_map, &camera, &atlas, &lighting);

        // Sprite behind wall at (4.2, 1.5): should be 100% occluded
        let occluded_sprite = [Sprite {
            pos: Vec2::new(4.2, 1.5),
            kind: SpriteKind::EnergyOrb,
            phase: 0.0,
            patrol_dir: Vec2::new(1.0, 0.0),
        }];
        let mut fb_occluded = fb_no_sprite.clone();
        render_sprites(&mut fb_occluded, &camera, &occluded_sprite, &atlas, &lighting, 0.0);
        assert_eq!(fb_occluded.pixels, fb_no_sprite.pixels);

        // Sprite in front of wall at (2.2, 1.5): must modify center pixels
        let visible_sprite = [Sprite {
            pos: Vec2::new(2.2, 1.5),
            kind: SpriteKind::EnergyOrb,
            phase: 0.0,
            patrol_dir: Vec2::new(1.0, 0.0),
        }];
        let mut fb_visible = fb_no_sprite.clone();
        render_sprites(&mut fb_visible, &camera, &visible_sprite, &atlas, &lighting, 0.0);
        assert_ne!(fb_visible.pixels, fb_no_sprite.pixels);
    }

    #[test]
    fn cyber_drone_patrols_and_reverses_at_walls() {
        let map = Map::parse("patrol", "#####\n#P.C#\n#####", 0.0).unwrap();
        let mut sprites: Vec<Sprite> = map
            .sprite_spawns
            .iter()
            .enumerate()
            .map(|(i, s)| Sprite::from_spawn(*s, i))
            .collect();
        sprites[0].patrol_dir = Vec2::new(1.0, 0.0);
        let initial_dir = sprites[0].patrol_dir;

        for _ in 0..30 {
            update_sprites(&mut sprites, &map, 0.05);
        }
        assert_ne!(sprites[0].patrol_dir.x, initial_dir.x);
    }
}

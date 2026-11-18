use crate::map::{Map, Tile};
use crate::math::{Camera, Rgb, Vec2};
use crate::raycaster::{cast_ray, Framebuffer};
use crate::sprite::Sprite;
use crate::textures::{SpriteKind, WallKind};

pub struct HudStatus<'a> {
    pub map_name: &'a str,
    pub fps: f32,
    pub camera: &'a Camera,
    pub flashlight_on: bool,
    pub minimap_on: bool,
    pub term_cols: usize,
}

pub fn draw_minimap(fb: &mut Framebuffer, map: &Map, camera: &Camera, sprites: &[Sprite]) {
    if fb.width < 48 || fb.height < 32 {
        return;
    }

    let max_w = fb.width / 3;
    let max_h = fb.height / 2;
    let cell_px = (max_w / map.width).min(max_h / map.height).clamp(2, 5);
    let mini_w = map.width * cell_px;
    let mini_h = map.height * cell_px;
    if mini_w + 6 > fb.width || mini_h + 6 > fb.height {
        return;
    }

    let origin_x = fb.width - mini_w - 3;
    let origin_y = 2;

    // Draw dark border frame
    for py in (origin_y - 1)..=(origin_y + mini_h) {
        for px in (origin_x - 1)..=(origin_x + mini_w) {
            let is_border = py == origin_y - 1
                || py == origin_y + mini_h
                || px == origin_x - 1
                || px == origin_x + mini_w;
            let color = if is_border {
                Rgb::new(70, 140, 175)
            } else {
                Rgb::new(12, 16, 22)
            };
            fb.set_pixel(px, py, color);
        }
    }

    // Draw map cells
    for gy in 0..map.height {
        for gx in 0..map.width {
            let cell_color = match map.tile_at(gx as i32, gy as i32) {
                Tile::Empty => None,
                Tile::Wall(WallKind::Brick) => Some(Rgb::new(150, 72, 56)),
                Tile::Wall(WallKind::TechMetal) => Some(Rgb::new(82, 128, 168)),
                Tile::Wall(WallKind::Hazard) => Some(Rgb::new(210, 170, 34)),
                Tile::Wall(WallKind::Stone) => Some(Rgb::new(105, 110, 118)),
                Tile::Wall(WallKind::Door) => Some(Rgb::new(220, 125, 40)),
                Tile::Door(idx) => {
                    if map.doors[idx].is_passable() {
                        Some(Rgb::new(45, 130, 80))
                    } else {
                        Some(Rgb::new(230, 130, 40))
                    }
                }
            };
            if let Some(color) = cell_color {
                for dy in 0..cell_px {
                    for dx in 0..cell_px {
                        fb.set_pixel(
                            origin_x + gx * cell_px + dx,
                            origin_y + gy * cell_px + dy,
                            color,
                        );
                    }
                }
            }
        }
    }

    // Draw FOV cone rays onto minimap
    let cone_rays = 24;
    for i in 0..=cone_rays {
        let camera_x = 2.0 * (i as f32 / cone_rays as f32) - 1.0;
        let ray_dir = Vec2::new(
            camera.dir.x + camera.plane.x * camera_x,
            camera.dir.y + camera.plane.y * camera_x,
        );
        let max_dist = cast_ray(map, camera.pos, ray_dir)
            .map_or(5.5, |h| h.perp_dist)
            .min(5.5);

        let steps = (max_dist * cell_px as f32 * 2.0) as usize;
        for s in 1..=steps {
            let d = (s as f32 / (cell_px as f32 * 2.0)).min(max_dist);
            let wx = camera.pos.x + ray_dir.x * d;
            let wy = camera.pos.y + ray_dir.y * d;
            let px = origin_x as i32 + (wx * cell_px as f32) as i32;
            let py = origin_y as i32 + (wy * cell_px as f32) as i32;
            if px >= origin_x as i32
                && px < (origin_x + mini_w) as i32
                && py >= origin_y as i32
                && py < (origin_y + mini_h) as i32
            {
                let prev = fb.get_pixel(px as usize, py as usize);
                fb.set_pixel(
                    px as usize,
                    py as usize,
                    prev.lerp(Rgb::new(60, 220, 160), 0.35),
                );
            }
        }
    }

    // Draw sprites on minimap
    for sprite in sprites {
        let px = origin_x as i32 + (sprite.pos.x * cell_px as f32) as i32;
        let py = origin_y as i32 + (sprite.pos.y * cell_px as f32) as i32;
        if px >= origin_x as i32
            && px < (origin_x + mini_w) as i32
            && py >= origin_y as i32
            && py < (origin_y + mini_h) as i32
        {
            let col = match sprite.kind {
                SpriteKind::EnergyOrb => Rgb::new(80, 240, 255),
                SpriteKind::CyberDrone => Rgb::new(255, 65, 65),
                SpriteKind::Pillar => Rgb::new(185, 190, 200),
            };
            fb.set_pixel(px as usize, py as usize, col);
        }
    }

    // Draw player marker
    let p_x = origin_x + ((camera.pos.x * cell_px as f32) as usize).min(mini_w - 1);
    let p_y = origin_y + ((camera.pos.y * cell_px as f32) as usize).min(mini_h - 1);
    for dy in 0..=1 {
        for dx in 0..=1 {
            let rx = (p_x + dx).min(origin_x + mini_w - 1);
            let ry = (p_y + dy).min(origin_y + mini_h - 1);
            fb.set_pixel(rx, ry, Rgb::new(255, 255, 255));
        }
    }
}

pub fn format_hud_line(status: &HudStatus<'_>) -> String {
    let light = if status.flashlight_on { "ON" } else { "OFF" };
    let mini = if status.minimap_on { "ON" } else { "OFF" };
    let raw = format!(
        " DOOM-TERM | {} | FPS:{:>5.1} | POS:({:.1},{:.1}) | ANG:{:>3.0}° | LIGHT:{}[F] | MAP:{}[M] | WASD/Arrows Q/C E:Door 1-2 Esc/X ",
        status.map_name,
        status.fps,
        status.camera.pos.x,
        status.camera.pos.y,
        status.camera.angle_deg(),
        light,
        mini,
    );
    let trimmed: String = raw.chars().take(status.term_cols).collect();
    format!(
        "\x1b[48;2;20;24;32m\x1b[38;2;225;232;240m{trimmed:<width$}\x1b[0m",
        width = status.term_cols
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_draws_into_top_right_corner() {
        let map = Map::preset(1);
        let camera = Camera::from_angle(map.spawn_pos, 0.0, 0.66);
        let mut fb = Framebuffer::new(160, 96);
        let before = fb.pixels.clone();
        draw_minimap(&mut fb, &map, &camera, &[]);
        assert_ne!(fb.pixels, before);
        // Bottom-left remains untouched black
        assert_eq!(fb.get_pixel(0, 95), Rgb::BLACK);
    }

    #[test]
    fn hud_line_contains_telemetry_and_fits_terminal_width() {
        let cam = Camera::from_angle(Vec2::new(2.5, 3.5), 0.0, 0.66);
        let line = format_hud_line(&HudStatus {
            map_name: "Sector 01",
            fps: 60.0,
            camera: &cam,
            flashlight_on: true,
            minimap_on: true,
            term_cols: 80,
        });
        assert!(line.contains("DOOM-TERM"));
        assert!(line.contains("FPS: 60.0"));
        assert!(line.contains("LIGHT:ON[F]"));
    }
}

use rayon::prelude::*;

use crate::map::{Map, Tile};
use crate::math::{apply_lighting, Camera, LightingConfig, Rgb, Vec2};
use crate::textures::{TextureAtlas, WallKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub perp_dist: f32,
    pub wall_kind: WallKind,
    pub wall_x: f32,
    pub side: u8,
    pub cell: (i32, i32),
}

#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    /// Column-major RGB pixel buffer (`x * height + y`).
    pub pixels: Vec<Rgb>,
    /// 1D perpendicular depth buffer per screen column.
    pub z_buffer: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgb::BLACK; width * height],
            z_buffer: vec![f32::INFINITY; width],
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.pixels.resize(width * height, Rgb::BLACK);
            self.z_buffer.resize(width, f32::INFINITY);
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Rgb {
        self.pixels[x * self.height + y]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Rgb) {
        self.pixels[x * self.height + y] = color;
    }
}

pub fn cast_ray(map: &Map, origin: Vec2, ray_dir: Vec2) -> Option<RayHit> {
    const MAX_STEPS: usize = 128;

    let mut map_x = origin.x.floor() as i32;
    let mut map_y = origin.y.floor() as i32;

    let delta_dist_x = if ray_dir.x.abs() < 1e-8 {
        1e30
    } else {
        (1.0 / ray_dir.x).abs()
    };
    let delta_dist_y = if ray_dir.y.abs() < 1e-8 {
        1e30
    } else {
        (1.0 / ray_dir.y).abs()
    };

    let (step_x, mut side_dist_x) = if ray_dir.x < 0.0 {
        (-1, (origin.x - map_x as f32) * delta_dist_x)
    } else {
        (1, (map_x as f32 + 1.0 - origin.x) * delta_dist_x)
    };

    let (step_y, mut side_dist_y) = if ray_dir.y < 0.0 {
        (-1, (origin.y - map_y as f32) * delta_dist_y)
    } else {
        (1, (map_y as f32 + 1.0 - origin.y) * delta_dist_y)
    };

    for _ in 0..MAX_STEPS {
        let side = if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            0
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            1
        };

        let tile = map.tile_at(map_x, map_y);
        if matches!(tile, Tile::Empty) {
            continue;
        }

        let perp_dist = if side == 0 {
            (side_dist_x - delta_dist_x).max(1e-4)
        } else {
            (side_dist_y - delta_dist_y).max(1e-4)
        };

        let hit_coord = if side == 0 {
            origin.y + perp_dist * ray_dir.y
        } else {
            origin.x + perp_dist * ray_dir.x
        };
        let wall_x = (hit_coord - hit_coord.floor()).clamp(0.0, 0.999_99);

        match tile {
            Tile::Empty => {}
            Tile::Wall(wall_kind) => {
                return Some(RayHit {
                    perp_dist,
                    wall_kind,
                    wall_x,
                    side,
                    cell: (map_x, map_y),
                });
            }
            Tile::Door(idx) => {
                let open = map.doors[idx].open_amount;
                if open < 0.98 && wall_x >= open {
                    return Some(RayHit {
                        perp_dist,
                        wall_kind: WallKind::Door,
                        wall_x: wall_x - open,
                        side,
                        cell: (map_x, map_y),
                    });
                }
            }
        }
    }

    None
}

pub fn render_world(
    fb: &mut Framebuffer,
    map: &Map,
    camera: &Camera,
    atlas: &TextureAtlas,
    lighting: &LightingConfig,
) {
    let width = fb.width;
    let height = fb.height;
    if width == 0 || height == 0 {
        return;
    }

    let half_h = height as f32 * 0.5;
    let inv_w = 1.0 / width as f32;

    fb.pixels
        .par_chunks_mut(height)
        .zip(fb.z_buffer.par_iter_mut())
        .enumerate()
        .for_each(|(x, (col, z_slot))| {
            let camera_x = 2.0 * (x as f32 + 0.5) * inv_w - 1.0;
            let ray_dir = Vec2::new(
                camera.dir.x + camera.plane.x * camera_x,
                camera.dir.y + camera.plane.y * camera_x,
            );

            let hit = cast_ray(map, camera.pos, ray_dir);
            let perp_dist = hit.map_or(1e6, |h| h.perp_dist);
            *z_slot = perp_dist;

            let line_height = (height as f32 / perp_dist).max(1.0);
            let wall_top_f = (height as f32 - line_height) * 0.5;
            let draw_start = (wall_top_f.floor() as i32).max(0) as usize;
            let draw_end = ((wall_top_f + line_height).ceil() as i32).min(height as i32) as usize;

            for (y, pixel) in col.iter_mut().enumerate() {
                let screen_y_norm = (y as f32 - half_h) / half_h;
                if y < draw_start {
                    let p = (half_h - y as f32).max(0.5);
                    let row_dist = half_h / p;
                    let wx = camera.pos.x + row_dist * ray_dir.x;
                    let wy = camera.pos.y + row_dist * ray_dir.y;
                    let base = atlas.sample_ceiling(wx, wy);
                    *pixel = apply_lighting(base, row_dist, camera_x, screen_y_norm, 0.9, lighting);
                } else if y < draw_end {
                    if let Some(h) = hit {
                        let v = ((y as f32 - wall_top_f) / line_height).clamp(0.0, 0.999_99);
                        let mut u = h.wall_x;
                        if (h.side == 0 && ray_dir.x > 0.0) || (h.side == 1 && ray_dir.y < 0.0) {
                            u = 1.0 - u;
                        }
                        let base = atlas.sample_wall(h.wall_kind, u, v);
                        let side_shade = if h.side == 1 { 0.72 } else { 1.0 };
                        *pixel = apply_lighting(
                            base,
                            h.perp_dist,
                            camera_x,
                            screen_y_norm,
                            side_shade,
                            lighting,
                        );
                    } else {
                        *pixel = lighting.fog_color;
                    }
                } else {
                    let p = (y as f32 - half_h).max(0.5);
                    let row_dist = half_h / p;
                    let wx = camera.pos.x + row_dist * ray_dir.x;
                    let wy = camera.pos.y + row_dist * ray_dir.y;
                    let base = atlas.sample_floor(wx, wy);
                    *pixel = apply_lighting(base, row_dist, camera_x, screen_y_norm, 1.0, lighting);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dda_hits_wall_at_exact_perpendicular_distance() {
        let map = Map::parse("room", "#####\n#P..#\n#####", 0.0).unwrap();
        // Player at (1.5, 1.5), wall at x = 4.0 -> perp_dist = 2.5
        let hit = cast_ray(&map, map.spawn_pos, Vec2::new(1.0, 0.0)).expect("should hit wall");
        assert!((hit.perp_dist - 2.5).abs() < 1e-4);
        assert_eq!(hit.wall_kind, WallKind::Brick);
        assert_eq!(hit.side, 0);
        assert_eq!(hit.cell, (4, 1));
    }

    #[test]
    fn dda_passes_through_open_door_to_wall_behind() {
        let mut map = Map::parse("door_ray", "######\n#P.D.#\n######", 0.0).unwrap();
        let closed_hit = cast_ray(&map, map.spawn_pos, Vec2::new(1.0, 0.0)).unwrap();
        assert_eq!(closed_hit.wall_kind, WallKind::Door);
        assert!((closed_hit.perp_dist - 1.5).abs() < 1e-4);

        map.doors[0].open_amount = 1.0;
        let open_hit = cast_ray(&map, map.spawn_pos, Vec2::new(1.0, 0.0)).unwrap();
        assert_eq!(open_hit.wall_kind, WallKind::Brick);
        assert!((open_hit.perp_dist - 3.5).abs() < 1e-4);
    }

    #[test]
    fn dda_partially_open_door_allows_partial_passthrough() {
        let mut map = Map::parse("partial_door", "######\n#P.D.#\n######", 0.0).unwrap();
        map.doors[0].open_amount = 0.6;
        // Origin at (1.5, 1.2) -> wall_x at x=3 is 0.2 < 0.6 (open gap!)
        let pass_hit = cast_ray(&map, Vec2::new(1.5, 1.2), Vec2::new(1.0, 0.0)).unwrap();
        assert_eq!(pass_hit.wall_kind, WallKind::Brick);

        // Origin at (1.5, 1.8) -> wall_x at x=3 is 0.8 >= 0.6 (hits sliding door!)
        let door_hit = cast_ray(&map, Vec2::new(1.5, 1.8), Vec2::new(1.0, 0.0)).unwrap();
        assert_eq!(door_hit.wall_kind, WallKind::Door);
    }
}

use crate::math::Vec2;
use crate::textures::{SpriteKind, WallKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorState {
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Door {
    pub x: usize,
    pub y: usize,
    pub state: DoorState,
    pub open_amount: f32,
    pub auto_close_timer: f32,
}

impl Door {
    pub const fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            state: DoorState::Closed,
            open_amount: 0.0,
            auto_close_timer: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.state = match self.state {
            DoorState::Closed | DoorState::Closing => DoorState::Opening,
            DoorState::Open | DoorState::Opening => DoorState::Closing,
        };
    }

    pub fn update(&mut self, dt: f32, player_in_doorway: bool) {
        const DOOR_SPEED: f32 = 1.85;
        match self.state {
            DoorState::Closed => {}
            DoorState::Opening => {
                self.open_amount = (self.open_amount + DOOR_SPEED * dt).min(1.0);
                if self.open_amount >= 1.0 {
                    self.open_amount = 1.0;
                    self.state = DoorState::Open;
                    self.auto_close_timer = 5.5;
                }
            }
            DoorState::Open => {
                if !player_in_doorway {
                    self.auto_close_timer -= dt;
                    if self.auto_close_timer <= 0.0 {
                        self.state = DoorState::Closing;
                    }
                }
            }
            DoorState::Closing => {
                if player_in_doorway {
                    self.state = DoorState::Opening;
                } else {
                    self.open_amount = (self.open_amount - DOOR_SPEED * dt).max(0.0);
                    if self.open_amount <= 0.0 {
                        self.open_amount = 0.0;
                        self.state = DoorState::Closed;
                    }
                }
            }
        }
    }

    pub fn is_passable(&self) -> bool {
        self.open_amount >= 0.75
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Wall(WallKind),
    Door(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteSpawn {
    pub x: f32,
    pub y: f32,
    pub kind: SpriteKind,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub doors: Vec<Door>,
    pub spawn_pos: Vec2,
    pub spawn_angle: f32,
    pub sprite_spawns: Vec<SpriteSpawn>,
}

impl Map {
    pub fn parse(name: &str, layout: &str, spawn_angle: f32) -> Result<Self, String> {
        let lines: Vec<&str> = layout
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.trim().is_empty())
            .collect();

        if lines.is_empty() {
            return Err("map layout is empty".to_string());
        }

        let height = lines.len();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        if width == 0 {
            return Err("map width is zero".to_string());
        }

        let mut tiles = vec![Tile::Empty; width * height];
        let mut doors = Vec::new();
        let mut sprite_spawns = Vec::new();
        let mut spawn_pos = None;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let tile = match ch {
                    '.' | ' ' => Tile::Empty,
                    '#' => Tile::Wall(WallKind::Brick),
                    'T' => Tile::Wall(WallKind::TechMetal),
                    'H' => Tile::Wall(WallKind::Hazard),
                    'S' => Tile::Wall(WallKind::Stone),
                    'D' => {
                        let idx = doors.len();
                        doors.push(Door::new(x, y));
                        Tile::Door(idx)
                    }
                    'P' => {
                        spawn_pos = Some(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
                        Tile::Empty
                    }
                    'O' => {
                        sprite_spawns.push(SpriteSpawn {
                            x: x as f32 + 0.5,
                            y: y as f32 + 0.5,
                            kind: SpriteKind::EnergyOrb,
                        });
                        Tile::Empty
                    }
                    'I' => {
                        sprite_spawns.push(SpriteSpawn {
                            x: x as f32 + 0.5,
                            y: y as f32 + 0.5,
                            kind: SpriteKind::Pillar,
                        });
                        Tile::Empty
                    }
                    'C' => {
                        sprite_spawns.push(SpriteSpawn {
                            x: x as f32 + 0.5,
                            y: y as f32 + 0.5,
                            kind: SpriteKind::CyberDrone,
                        });
                        Tile::Empty
                    }
                    other => {
                        return Err(format!("unknown map tile '{other}' at ({x}, {y})"));
                    }
                };
                tiles[y * width + x] = tile;
            }
        }

        let spawn_pos = spawn_pos.ok_or_else(|| "map missing player spawn 'P'".to_string())?;

        Ok(Self {
            name: name.to_string(),
            width,
            height,
            tiles,
            doors,
            spawn_pos,
            spawn_angle,
            sprite_spawns,
        })
    }

    pub fn preset(id: usize) -> Self {
        match id {
            2 => Self::parse("Sector 02: Reactor Core", MAP_2, 0.0)
                .expect("built-in map 2 must parse"),
            _ => Self::parse("Sector 01: Tech Bunker", MAP_1, 0.0)
                .expect("built-in map 1 must parse"),
        }
    }

    pub fn tile_at(&self, gx: i32, gy: i32) -> Tile {
        if gx < 0 || gy < 0 {
            return Tile::Wall(WallKind::Stone);
        }
        let ux = gx as usize;
        let uy = gy as usize;
        if ux >= self.width || uy >= self.height {
            return Tile::Wall(WallKind::Stone);
        }
        self.tiles[uy * self.width + ux]
    }

    pub fn is_blocking(&self, wx: f32, wy: f32) -> bool {
        let gx = wx.floor() as i32;
        let gy = wy.floor() as i32;
        match self.tile_at(gx, gy) {
            Tile::Empty => false,
            Tile::Wall(_) => true,
            Tile::Door(idx) => !self.doors[idx].is_passable(),
        }
    }

    pub fn move_with_collision(&self, pos: Vec2, delta: Vec2, radius: f32) -> Vec2 {
        let mut next = pos;

        let try_x = pos.x + delta.x;
        if !self.circle_blocked(try_x, pos.y, radius) {
            next.x = try_x;
        }

        let try_y = pos.y + delta.y;
        if !self.circle_blocked(next.x, try_y, radius) {
            next.y = try_y;
        }

        next
    }

    fn circle_blocked(&self, wx: f32, wy: f32, radius: f32) -> bool {
        for sx in [-radius, radius] {
            for sy in [-radius, radius] {
                if self.is_blocking(wx + sx, wy + sy) {
                    return true;
                }
            }
        }
        false
    }

    pub fn interact_nearest_door(&mut self, pos: Vec2, dir: Vec2) -> bool {
        for step in 1..=12 {
            let dist = step as f32 * 0.2;
            let px = pos.x + dir.x * dist;
            let py = pos.y + dir.y * dist;
            if let Tile::Door(idx) = self.tile_at(px.floor() as i32, py.floor() as i32) {
                self.doors[idx].toggle();
                return true;
            }
        }
        false
    }

    pub fn update_doors(&mut self, dt: f32, player_pos: Vec2) {
        let px = player_pos.x.floor() as i32;
        let py = player_pos.y.floor() as i32;
        for door in &mut self.doors {
            let in_doorway = door.x as i32 == px && door.y as i32 == py;
            door.update(dt, in_doorway);
        }
    }
}

const MAP_1: &str = "\
################
#P..#..O...T...#
#...D......T.I.#
#...#..C...D...#
###D#####H#T####
#...S....H.....#
#.I.S.O..D..C..#
#...S....H.....#
#SSSS####HTTTT##
#......D.......#
#.O..C.#..I..O.#
#......#.......#
################
";

const MAP_2: &str = "\
TTTTTTTTTTTTTTTTTTTT
TP..T...O..H...C...T
T...D......D.......T
T.I.T..C...H..I..O.T
TTDTTTTTHHHHTTTTDTTT
#......S....S......#
#.O..I.D..C.D.I..O.#
#......S....S......#
###H####....####H###
S......D....D......S
S..C...S.OO.S...C..S
S......S....S......S
SSSSSSSSSSSSSSSSSSSS
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_map_and_built_in_presets() {
        let m1 = Map::preset(1);
        let m2 = Map::preset(2);
        assert!(m1.width >= 16 && m1.height >= 12);
        assert!(m2.width >= 20 && m2.height >= 12);
        assert!(!m1.doors.is_empty());
        assert!(!m2.doors.is_empty());
        assert!(!m1.sprite_spawns.is_empty());
        assert!(!m2.sprite_spawns.is_empty());
    }

    #[test]
    fn rejects_invalid_maps() {
        assert!(Map::parse("empty", "", 0.0).is_err());
        assert!(Map::parse("no_spawn", "###\n#.#\n###", 0.0).is_err());
        assert!(Map::parse("bad_tile", "###\n#P?\n###", 0.0).is_err());
    }

    #[test]
    fn door_state_animation_transitions_cleanly() {
        let mut door = Door::new(3, 3);
        assert_eq!(door.state, DoorState::Closed);
        assert!(!door.is_passable());

        door.toggle();
        assert_eq!(door.state, DoorState::Opening);

        for _ in 0..20 {
            door.update(0.05, false);
        }
        assert_eq!(door.state, DoorState::Open);
        assert!((door.open_amount - 1.0).abs() < 1e-5);
        assert!(door.is_passable());

        door.toggle();
        assert_eq!(door.state, DoorState::Closing);
        for _ in 0..20 {
            door.update(0.05, false);
        }
        assert_eq!(door.state, DoorState::Closed);
        assert!(door.open_amount.abs() < 1e-5);
    }

    #[test]
    fn collision_blocks_closed_door_and_slides_along_walls() {
        let mut map = Map::parse("test", "#####\n#P.D#\n#####", 0.0).unwrap();
        let blocked = map.move_with_collision(Vec2::new(2.75, 1.5), Vec2::new(0.5, 0.0), 0.2);
        assert!(blocked.x < 3.0);

        map.doors[0].open_amount = 1.0;
        map.doors[0].state = DoorState::Open;
        let passed = map.move_with_collision(Vec2::new(2.75, 1.5), Vec2::new(0.5, 0.0), 0.2);
        assert!(passed.x > 3.0);
    }
}

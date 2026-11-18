use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};

use crate::ansi::{encode_half_blocks, EncodeResult};
use crate::hud::{draw_minimap, format_hud_line, HudStatus};
use crate::map::Map;
use crate::math::{Camera, LightingConfig, Vec2};
use crate::raycaster::{render_world, Framebuffer};
use crate::sprite::{render_sprites, update_sprites, Sprite};
use crate::textures::TextureAtlas;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub headless: bool,
    pub frames: usize,
    pub width: usize,
    pub height: usize,
    pub benchmark: bool,
    pub map_id: usize,
    pub show_help: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            headless: false,
            frames: 60,
            width: 120,
            height: 40,
            benchmark: false,
            map_id: 1,
            show_help: false,
        }
    }
}

pub fn parse_args(args: &[String]) -> Result<CliConfig, String> {
    let mut cfg = CliConfig::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" => {
                cfg.show_help = true;
            }
            "--headless" => {
                cfg.headless = true;
            }
            "--benchmark" => {
                cfg.benchmark = true;
            }
            "--frames" => {
                i += 1;
                let val = args.get(i).ok_or("--frames requires a value")?;
                cfg.frames = parse_positive_usize(val, "--frames")?;
            }
            "--width" => {
                i += 1;
                let val = args.get(i).ok_or("--width requires a value")?;
                cfg.width = parse_positive_usize(val, "--width")?;
            }
            "--height" => {
                i += 1;
                let val = args.get(i).ok_or("--height requires a value")?;
                cfg.height = parse_positive_usize(val, "--height")?;
            }
            "--map" => {
                i += 1;
                let val = args.get(i).ok_or("--map requires a value (1 or 2)")?;
                cfg.map_id = parse_map_id(val)?;
            }
            _ if arg.starts_with("--frames=") => {
                cfg.frames = parse_positive_usize(&arg["--frames=".len()..], "--frames")?;
            }
            _ if arg.starts_with("--width=") => {
                cfg.width = parse_positive_usize(&arg["--width=".len()..], "--width")?;
            }
            _ if arg.starts_with("--height=") => {
                cfg.height = parse_positive_usize(&arg["--height=".len()..], "--height")?;
            }
            _ if arg.starts_with("--map=") => {
                cfg.map_id = parse_map_id(&arg["--map=".len()..])?;
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        i += 1;
    }
    Ok(cfg)
}

fn parse_positive_usize(raw: &str, flag: &str) -> Result<usize, String> {
    let v: usize = raw
        .parse()
        .map_err(|_| format!("invalid integer for {flag}: '{raw}'"))?;
    if v == 0 {
        return Err(format!("{flag} must be >= 1"));
    }
    Ok(v)
}

fn parse_map_id(raw: &str) -> Result<usize, String> {
    match raw {
        "1" => Ok(1),
        "2" => Ok(2),
        _ => Err(format!("invalid map '{raw}' (expected 1 or 2)")),
    }
}

pub fn help_text() -> &'static str {
    "\
doom-term — Textured Wolfenstein/DOOM Terminal Raycaster

USAGE:
    doom-term [OPTIONS]

OPTIONS:
    --map <1|2>       Select starting sector (default: 1)
    --headless        Run without TTY raw mode (renders frames in memory)
    --frames <N>      Number of frames to render in headless mode (default: 60)
    --width <W>       Terminal column width in headless mode (default: 120)
    --height <H>      Terminal row height in headless mode (default: 40)
    --benchmark       Print FPS and ANSI deduplication telemetry
    -h, --help        Show this help message

CONTROLS (Interactive):
    W / S, Up / Down  Move forward / backward
    A / D, Left/Right Turn left / right
    Q / C             Strafe left / right
    E / Space         Open / close sliding door ahead
    F                 Toggle dynamic flashlight cone
    M                 Toggle corner minimap & FOV cone
    1 / 2             Switch between Sector 01 and Sector 02
    Esc / X           Quit"
}

pub struct GameState {
    pub map: Map,
    pub map_id: usize,
    pub camera: Camera,
    pub sprites: Vec<Sprite>,
    pub atlas: TextureAtlas,
    pub lighting: LightingConfig,
    pub minimap_on: bool,
    pub time: f32,
}

impl GameState {
    pub fn new(map_id: usize) -> Self {
        let map = Map::preset(map_id);
        let camera = Camera::from_angle(map.spawn_pos, map.spawn_angle, 0.66);
        let sprites = map
            .sprite_spawns
            .iter()
            .enumerate()
            .map(|(i, s)| Sprite::from_spawn(*s, i))
            .collect();
        Self {
            map,
            map_id,
            camera,
            sprites,
            atlas: TextureAtlas::new(),
            lighting: LightingConfig::default(),
            minimap_on: true,
            time: 0.0,
        }
    }

    pub fn load_map(&mut self, map_id: usize) {
        let map = Map::preset(map_id);
        self.map_id = map_id;
        self.camera = Camera::from_angle(map.spawn_pos, map.spawn_angle, 0.66);
        self.sprites = map
            .sprite_spawns
            .iter()
            .enumerate()
            .map(|(i, s)| Sprite::from_spawn(*s, i))
            .collect();
        self.map = map;
    }

    pub fn step(&mut self, dt: f32) {
        self.time += dt;
        self.map.update_doors(dt, self.camera.pos);
        update_sprites(&mut self.sprites, &self.map, dt);
    }

    pub fn move_forward(&mut self, dist: f32) {
        let delta = Vec2::new(self.camera.dir.x * dist, self.camera.dir.y * dist);
        self.camera.pos = self.map.move_with_collision(self.camera.pos, delta, 0.22);
    }

    pub fn strafe(&mut self, dist: f32) {
        let right = Vec2::new(-self.camera.dir.y, self.camera.dir.x);
        let delta = Vec2::new(right.x * dist, right.y * dist);
        self.camera.pos = self.map.move_with_collision(self.camera.pos, delta, 0.22);
    }

    pub fn render_frame(
        &self,
        fb: &mut Framebuffer,
        fps: f32,
        term_cols: usize,
    ) -> (String, EncodeResult) {
        render_world(fb, &self.map, &self.camera, &self.atlas, &self.lighting);
        render_sprites(
            fb,
            &self.camera,
            &self.sprites,
            &self.atlas,
            &self.lighting,
            self.time,
        );
        if self.minimap_on {
            draw_minimap(fb, &self.map, &self.camera, &self.sprites);
        }

        let (mut ansi, encode_stats) = encode_half_blocks(fb);
        let hud = format_hud_line(&HudStatus {
            map_name: &self.map.name,
            fps,
            camera: &self.camera,
            flashlight_on: self.lighting.flashlight_on,
            minimap_on: self.minimap_on,
            term_cols,
        });
        ansi.push_str("\r\n");
        ansi.push_str(&hud);
        (ansi, encode_stats)
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub frames: usize,
    pub term_cols: usize,
    pub term_rows: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub map_name: String,
    pub elapsed_secs: f64,
    pub fps: f64,
    pub avg_ms: f64,
    pub total_bytes: usize,
    pub avg_bytes: usize,
    pub escape_reduction_pct: f64,
    pub last_frame_len: usize,
}

pub fn run_headless(cfg: &CliConfig) -> BenchmarkReport {
    let mut game = GameState::new(cfg.map_id);
    let pixel_height = cfg.height * 2;
    let mut fb = Framebuffer::new(cfg.width, pixel_height);

    let mut total_bytes = 0usize;
    let mut total_escapes = 0usize;
    let mut total_naive_escapes = 0usize;
    let mut last_frame_len = 0usize;

    let start = Instant::now();
    for frame_idx in 0..cfg.frames {
        if frame_idx == cfg.frames / 3 {
            let _ = game
                .map
                .interact_nearest_door(game.camera.pos, game.camera.dir);
        }
        game.camera.rotate(0.035);
        game.move_forward(0.02);
        game.step(1.0 / 60.0);

        let (frame, enc) = game.render_frame(&mut fb, 60.0, cfg.width);
        total_bytes += enc.bytes;
        total_escapes += enc.color_escapes;
        total_naive_escapes += enc.naive_color_escapes;
        last_frame_len = frame.len();
    }
    let elapsed_secs = start.elapsed().as_secs_f64().max(1e-9);
    let fps = cfg.frames as f64 / elapsed_secs;
    let avg_ms = (elapsed_secs * 1000.0) / cfg.frames as f64;
    let avg_bytes = total_bytes / cfg.frames.max(1);
    let escape_reduction_pct = if total_naive_escapes > 0 {
        (1.0 - (total_escapes as f64 / total_naive_escapes as f64)) * 100.0
    } else {
        0.0
    };

    BenchmarkReport {
        frames: cfg.frames,
        term_cols: cfg.width,
        term_rows: cfg.height,
        pixel_width: cfg.width,
        pixel_height,
        map_name: game.map.name,
        elapsed_secs,
        fps,
        avg_ms,
        total_bytes,
        avg_bytes,
        escape_reduction_pct,
        last_frame_len,
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

pub fn run_interactive(cfg: &CliConfig) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let _guard = TerminalGuard;

    let (mut cols, mut rows) = terminal::size()?;
    let mut game = GameState::new(cfg.map_id);
    let mut fb = Framebuffer::new(
        usize::from(cols).max(16),
        (usize::from(rows).saturating_sub(1).max(8)) * 2,
    );

    let mut last_tick = Instant::now();
    let mut smoothed_fps = 60.0f32;
    let mut running = true;

    while running {
        let now = Instant::now();
        let dt = now
            .duration_since(last_tick)
            .as_secs_f32()
            .clamp(0.001, 0.1);
        last_tick = now;

        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Resize(new_cols, new_rows) => {
                    cols = new_cols;
                    rows = new_rows;
                    let w = usize::from(cols).max(16);
                    let h = usize::from(rows).saturating_sub(1).max(8) * 2;
                    fb.resize(w, h);
                }
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    let move_step = 0.22;
                    let turn_step = 0.085;
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('x') | KeyCode::Char('X') => {
                            running = false;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            running = false;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                            game.move_forward(move_step);
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                            game.move_forward(-move_step);
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                            game.camera.rotate(-turn_step);
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                            game.camera.rotate(turn_step);
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            game.strafe(-move_step);
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            game.strafe(move_step);
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Char(' ') => {
                            let _ = game
                                .map
                                .interact_nearest_door(game.camera.pos, game.camera.dir);
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            game.lighting.flashlight_on = !game.lighting.flashlight_on;
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            game.minimap_on = !game.minimap_on;
                        }
                        KeyCode::Char('1') => {
                            game.load_map(1);
                        }
                        KeyCode::Char('2') => {
                            game.load_map(2);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        game.step(dt);
        let render_start = Instant::now();
        let (frame, _) = game.render_frame(&mut fb, smoothed_fps, usize::from(cols).max(16));
        let frame_secs = render_start.elapsed().as_secs_f32().max(1e-5);
        let inst_fps = (1.0 / dt.max(frame_secs)).min(999.9);
        smoothed_fps = smoothed_fps * 0.85 + inst_fps * 0.15;

        stdout.write_all(b"\x1b[H")?;
        stdout.write_all(frame.as_bytes())?;
        stdout.flush()?;

        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headless_and_benchmark_cli_flags() {
        let args: Vec<String> = vec![
            "--headless",
            "--frames",
            "15",
            "--width=160",
            "--height",
            "48",
            "--map",
            "2",
            "--benchmark",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let cfg = parse_args(&args).unwrap();
        assert!(cfg.headless);
        assert!(cfg.benchmark);
        assert_eq!(cfg.frames, 15);
        assert_eq!(cfg.width, 160);
        assert_eq!(cfg.height, 48);
        assert_eq!(cfg.map_id, 2);
    }

    #[test]
    fn rejects_invalid_cli_flags() {
        let bad_frames = vec!["--frames".into(), "0".into()];
        assert!(parse_args(&bad_frames).is_err());

        let bad_map = vec!["--map".into(), "9".into()];
        assert!(parse_args(&bad_map).is_err());

        let unknown = vec!["--bogus".into()];
        assert!(parse_args(&unknown).is_err());
    }

    #[test]
    fn headless_runner_completes_and_reports_metrics() {
        let cfg = CliConfig {
            headless: true,
            frames: 5,
            width: 80,
            height: 24,
            benchmark: true,
            map_id: 1,
            show_help: false,
        };
        let report = run_headless(&cfg);
        assert_eq!(report.frames, 5);
        assert_eq!(report.pixel_width, 80);
        assert_eq!(report.pixel_height, 48);
        assert!(report.fps > 0.0);
        assert!(report.avg_bytes > 0);
        assert!(report.last_frame_len > 0);
    }
}

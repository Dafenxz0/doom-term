use rayon::prelude::*;

use crate::math::Rgb;
use crate::raycaster::Framebuffer;

#[derive(Debug, Clone, Copy, Default)]
pub struct EncodeResult {
    pub bytes: usize,
    pub color_escapes: usize,
    pub naive_color_escapes: usize,
}

#[inline]
fn push_u8(out: &mut String, val: u8) {
    if val >= 100 {
        out.push((b'0' + val / 100) as char);
        out.push((b'0' + (val / 10) % 10) as char);
        out.push((b'0' + val % 10) as char);
    } else if val >= 10 {
        out.push((b'0' + val / 10) as char);
        out.push((b'0' + val % 10) as char);
    } else {
        out.push((b'0' + val) as char);
    }
}

#[inline]
fn push_rgb_escape(out: &mut String, prefix: &str, c: Rgb) {
    out.push_str(prefix);
    push_u8(out, c.r);
    out.push(';');
    push_u8(out, c.g);
    out.push(';');
    push_u8(out, c.b);
    out.push('m');
}

fn encode_row(fb: &Framebuffer, row: usize) -> (String, usize) {
    let y_top = row * 2;
    let y_bot = y_top + 1;
    let mut line = String::with_capacity(fb.width * 10);
    let mut prev_fg: Option<Rgb> = None;
    let mut prev_bg: Option<Rgb> = None;
    let mut escapes = 0usize;

    for x in 0..fb.width {
        let top = fb.get_pixel(x, y_top);
        let bot = fb.get_pixel(x, y_bot);

        if top == bot {
            if prev_bg != Some(bot) {
                push_rgb_escape(&mut line, "\x1b[48;2;", bot);
                prev_bg = Some(bot);
                escapes += 1;
            }
            line.push(' ');
        } else {
            if prev_fg != Some(top) {
                push_rgb_escape(&mut line, "\x1b[38;2;", top);
                prev_fg = Some(top);
                escapes += 1;
            }
            if prev_bg != Some(bot) {
                push_rgb_escape(&mut line, "\x1b[48;2;", bot);
                prev_bg = Some(bot);
                escapes += 1;
            }
            line.push('▀');
        }
    }
    line.push_str("\x1b[0m");
    (line, escapes)
}

pub fn encode_half_blocks(fb: &Framebuffer) -> (String, EncodeResult) {
    let term_rows = fb.height / 2;
    if fb.width == 0 || term_rows == 0 {
        return (String::new(), EncodeResult::default());
    }

    let rows: Vec<(String, usize)> = (0..term_rows)
        .into_par_iter()
        .map(|row| encode_row(fb, row))
        .collect();

    let total_bytes: usize = rows.iter().map(|(s, _)| s.len() + 2).sum();
    let mut out = String::with_capacity(total_bytes + 8);
    let mut color_escapes = 0usize;

    for (idx, (line, esc)) in rows.into_iter().enumerate() {
        out.push_str(&line);
        color_escapes += esc;
        if idx + 1 < term_rows {
            out.push_str("\r\n");
        }
    }

    let naive_color_escapes = fb.width * term_rows * 2;
    let stats = EncodeResult {
        bytes: out.len(),
        color_escapes,
        naive_color_escapes,
    };
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_half_block_top_fg_and_bottom_bg() {
        let mut fb = Framebuffer::new(2, 2);
        let red = Rgb::new(255, 10, 20);
        let blue = Rgb::new(5, 30, 220);
        let green = Rgb::new(10, 200, 30);

        fb.set_pixel(0, 0, red);
        fb.set_pixel(0, 1, blue);
        fb.set_pixel(1, 0, green);
        fb.set_pixel(1, 1, blue);

        let (ansi, stats) = encode_half_blocks(&fb);
        assert!(ansi.contains("\x1b[38;2;255;10;20m"));
        assert!(ansi.contains("\x1b[48;2;5;30;220m"));
        assert!(ansi.contains("\x1b[38;2;10;200;30m"));
        assert!(ansi.contains('▀'));
        // Second cell keeps same bg (`blue`) so only 3 escapes instead of 4
        assert_eq!(stats.color_escapes, 3);
        assert_eq!(stats.naive_color_escapes, 4);
    }

    #[test]
    fn run_length_deduplication_compresses_solid_rows() {
        let mut fb = Framebuffer::new(20, 4);
        let steel = Rgb::new(40, 50, 60);
        for x in 0..20 {
            for y in 0..4 {
                fb.set_pixel(x, y, steel);
            }
        }
        let (ansi, stats) = encode_half_blocks(&fb);
        // Only 1 bg escape per terminal row (2 total) instead of 20 * 2 * 2 = 80
        assert_eq!(stats.color_escapes, 2);
        assert_eq!(stats.naive_color_escapes, 80);
        assert!(ansi.contains("\x1b[48;2;40;50;60m"));
    }

    #[test]
    fn handles_empty_and_odd_height_safely() {
        let empty = Framebuffer::new(0, 0);
        let (out, stats) = encode_half_blocks(&empty);
        assert!(out.is_empty());
        assert_eq!(stats.bytes, 0);

        let odd = Framebuffer::new(4, 1);
        let (out_odd, _) = encode_half_blocks(&odd);
        assert!(out_odd.is_empty());
    }
}

use std::process;

use doom_term::engine::{help_text, parse_args, run_headless, run_interactive};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = match parse_args(&args) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("error: {err}\n\n{}", help_text());
            process::exit(2);
        }
    };

    if cfg.show_help {
        println!("{}", help_text());
        return;
    }

    if cfg.headless {
        let report = run_headless(&cfg);
        if cfg.benchmark {
            println!(
                "doom-term headless benchmark\n\
                 Map:              {}\n\
                 Resolution:       {}x{} cells ({}x{} px half-block)\n\
                 Frames rendered:  {}\n\
                 Elapsed:          {:.2} ms\n\
                 Avg frame time:   {:.3} ms\n\
                 Throughput:       {:.1} FPS\n\
                 Avg ANSI frame:   {} bytes ({:.1}% color-escape reduction)",
                report.map_name,
                report.term_cols,
                report.term_rows,
                report.pixel_width,
                report.pixel_height,
                report.frames,
                report.elapsed_secs * 1000.0,
                report.avg_ms,
                report.fps,
                report.avg_bytes,
                report.escape_reduction_pct,
            );
        } else {
            println!(
                "doom-term headless ok: {} frames at {}x{} ({:.1} FPS, {} bytes/frame)",
                report.frames, report.term_cols, report.term_rows, report.fps, report.avg_bytes,
            );
        }
        return;
    }

    if let Err(err) = run_interactive(&cfg) {
        eprintln!("runtime error: {err}");
        process::exit(1);
    }
}

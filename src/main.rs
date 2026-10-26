mod app;
mod draw;
mod figlet;
mod font;
mod format;
mod parse;

use std::process::exit;

use clap::Parser;

use figlet::FigletFont;
use font::Font;
use format::format_seconds;

const DEFAULT_TIME_FORMAT: &str = "%H:%M:%S"; // --no-seconds expects this to end with :%S
const UNIVERS_FLF: &str = include_str!("../fonts/univers.flf");

const ABOUT: &str = "\
Starts a countdown to or from TIME. Example values for TIME:
10, '1h 5m 30s', '12:00', '2020-01-01', '2020-01-01 14:00 UTC'.

If TIME is not given, termdown will operate in stopwatch mode
and count forward.

Hotkeys:
    L       Lap (stopwatch mode only)
    R       Reset
    SPACE   Pause (will delay absolute TIME)
    +       Plus (will add 10 seconds)
    -       Minus (will subtract 10 seconds)
    Q       Quit";

#[derive(Parser)]
#[command(name = "termdown", version, about = ABOUT)]
pub struct Cli {
    /// Timer target; countdown mode when given, stopwatch mode when omitted
    #[arg(value_name = "TIME")]
    pub timespec: Option<String>,

    /// Use colon-separated time format
    #[arg(short = 'a', long)]
    pub alt_format: bool,

    /// Flash terminal at end of countdown
    #[arg(short = 'b', long)]
    pub blink: bool,

    /// Don't ring terminal bell at end of countdown
    #[arg(short = 'B', long)]
    pub no_bell: bool,

    /// Draw final N seconds in red and announce them individually with
    /// --voice or --exec-cmd (defaults to 3)
    #[arg(short = 'c', long, value_name = "N", default_value_t = 3)]
    pub critical: i64,

    /// 'block' (default) for the built-in pixel font, or a figlet font:
    /// choose from http://www.figlet.org/examples.html
    #[arg(short = 'f', long, value_name = "FONT", default_value = "block")]
    pub font: String,

    /// Add TEXT to the beginning of --voice and --exec annunciations
    /// (except per-second ones)
    #[arg(short = 'p', long, value_name = "TEXT")]
    pub voice_prefix: Option<String>,

    /// Quit N seconds after countdown (use with -b or -t) or terminate
    /// stopwatch after N seconds
    #[arg(short = 'q', long, value_name = "N")]
    pub quit_after: Option<f64>,

    /// Don't show seconds (except for last minute of countdown and first
    /// minute of stopwatch)
    #[arg(short = 's', long)]
    pub no_seconds: bool,

    /// Text to display at end of countdown
    #[arg(short = 't', long)]
    pub text: Option<String>,

    /// Text to display on top of countdown/stopwatch
    #[arg(short = 'T', long)]
    pub title: Option<String>,

    /// Don't update terminal title with remaining/elapsed time
    #[arg(short = 'W', long)]
    pub no_window_title: bool,

    /// Spoken countdown (at fixed intervals with per-second annunciations
    /// starting at --critical; requires `espeak` on Linux or `say` on macOS;
    /// e.g. `-v en-us` or `-v de` (espeak, list with `espeak --voices`),
    /// `-v Alex` or `-v Samantha` (say, list with `say -v '?'`)
    #[arg(short = 'v', long, value_name = "VOICE")]
    pub voice: Option<String>,

    /// File to write current remaining/elapsed time to
    #[arg(short = 'o', long, value_name = "PATH")]
    pub outfile: Option<std::path::PathBuf>,

    /// Runs CMD every second. '{0}' and '{1}' in CMD will be replaced with
    /// the remaining/elapsed number of seconds and a more sparse annunciation
    /// as in --voice, respectively
    #[arg(long, value_name = "CMD")]
    pub exec_cmd: Option<String>,

    /// Don't use ASCII art for display
    #[arg(long)]
    pub no_figlet: bool,

    /// Vertical offset within the terminal (only for --no-figlet)
    #[arg(long, value_name = "N", default_value_t = -1, allow_hyphen_values = true)]
    pub no_figlet_y_offset: i64,

    /// Don't try to replace non-ASCII characters (use with -t)
    #[arg(long)]
    pub no_text_magic: bool,

    /// Show current time instead of countdown/stopwatch
    #[arg(short = 'z', long = "time")]
    pub time: bool,

    /// Format for --time (defaults to "%H:%M:%S", ignores --no-seconds)
    #[arg(short = 'Z', long, value_name = "FORMAT")]
    pub time_format: Option<String>,
}

fn load_font(name: &str) -> Result<Font, String> {
    if name == "block" {
        return Ok(Font::Block);
    }
    if name == "univers" {
        return FigletFont::parse(UNIVERS_FLF).map(Font::Figlet);
    }
    let candidates = [
        name.to_string(),
        format!("{}.flf", name),
        format!("/usr/share/figlet/{}.flf", name),
        format!("/usr/share/figlet/{}.tlf", name),
    ];
    for candidate in &candidates {
        if !std::path::Path::new(candidate).is_file() {
            continue;
        }
        let bytes = std::fs::read(candidate)
            .map_err(|e| format!("Unable to read font '{}': {}", candidate, e))?;
        if bytes.starts_with(b"PK") {
            return Err(format!(
                "Font '{}' is a zipped figlet font, which is not supported",
                candidate
            ));
        }
        let data = String::from_utf8_lossy(&bytes);
        return FigletFont::parse(&data)
            .map(Font::Figlet)
            .map_err(|e| format!("Problem parsing font '{}': {}", name, e));
    }
    Err(format!(
        "Unable to find font '{}' (tried the path itself and /usr/share/figlet)",
        name
    ))
}

fn validate_time_format(fmt: &str) -> bool {
    use chrono::format::{Item, StrftimeItems};
    !StrftimeItems::new(fmt).any(|item| matches!(item, Item::Error))
}

fn finish(cli: &Cli) {
    if !cli.no_window_title {
        print!("\x1b]2;\x07");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    if let Some(outfile) = &cli.outfile {
        if outfile.exists() {
            let _ = std::fs::remove_file(outfile);
        }
    }
}

fn print_laps(seconds_elapsed: f64, laps: &[f64]) {
    for (lap_index, lap_time) in laps.iter().enumerate() {
        eprint!(
            "{:.3}\t{}\tlap {}\n",
            lap_time,
            format_seconds(*lap_time as i64, false),
            lap_index + 1
        );
    }
    if !laps.is_empty() {
        eprint!(
            "{:.3}\t{}\tlap {}\n",
            seconds_elapsed,
            format_seconds(seconds_elapsed as i64, false),
            laps.len() + 1
        );
        let mut all: Vec<f64> = laps.to_vec();
        all.push(seconds_elapsed);
        let total_seconds: f64 = all.iter().sum();
        let average_seconds = total_seconds / all.len() as f64;
        eprint!(
            "{:.3}\t{}\tlap avg\n",
            average_seconds,
            format_seconds(average_seconds as i64, false)
        );
        eprint!(
            "{:.3}\t{}\ttotal\n",
            total_seconds,
            format_seconds(total_seconds as i64, false)
        );
    } else {
        eprint!(
            "{:.3}\t{}\ttotal\n",
            seconds_elapsed,
            format_seconds(seconds_elapsed as i64, false)
        );
    }
}

fn main() {
    let cli = Cli::parse();
    let time_format = cli.time_format.clone().unwrap_or_else(|| {
        if cli.no_seconds {
            DEFAULT_TIME_FORMAT[..DEFAULT_TIME_FORMAT.len() - 3].to_string()
        } else {
            DEFAULT_TIME_FORMAT.to_string()
        }
    });
    if cli.time && !validate_time_format(&time_format) {
        eprintln!("Error: Invalid time format: {}", time_format);
        exit(2);
    }
    if let Some(outfile) = &cli.outfile {
        if outfile.exists() {
            eprintln!("Error: File already exists: {}", outfile.display());
            exit(2);
        }
    }
    let font = match load_font(&cli.font) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(2);
        }
    };
    if let Some(voice) = &cli.voice {
        if let Err(e) = app::check_voice(voice) {
            eprintln!("Error: {}", e);
            exit(2);
        }
    }

    match cli.timespec.clone() {
        Some(timespec) => {
            let parsed = match parse::parse_timestr(&timespec) {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("Error: Unable to parse TIME value '{}'", timespec);
                    exit(2);
                }
            };
            let mut terminal = ratatui::init();
            let result = app::countdown(&mut terminal, &cli, &font, &timespec, parsed, &time_format);
            ratatui::restore();
            finish(&cli);
            if let Some(voice_error) = app::VOICE_ERROR.lock().unwrap().take() {
                eprintln!("Error: {}", voice_error);
            }
            match result {
                Ok(code) => exit(code),
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
        None => {
            let mut terminal = ratatui::init();
            let result = app::stopwatch(&mut terminal, &cli, &font, &time_format);
            ratatui::restore();
            finish(&cli);
            match result {
                Ok((seconds_elapsed, laps, code)) => {
                    if code == 0 {
                        print_laps(seconds_elapsed, &laps);
                    }
                    exit(code);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }
    }
}

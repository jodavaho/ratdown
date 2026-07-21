// Structural port of termdown's countdown() and stopwatch() loops. The
// python version uses an input thread feeding a queue; here crossterm's
// event::poll provides the same timed-wait semantics inline.
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

use chrono::{Duration, Local, NaiveDateTime, Timelike};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Style};
use ratatui::DefaultTerminal;

use crate::draw;
use crate::font::Font;
use crate::format::{format_seconds, format_seconds_alt, normalize_text};
use crate::parse::parse_timestr;
use crate::Cli;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Input {
    Pause,
    Reset,
    Exit,
    Lap,
    Plus,
    Minus,
    CtrlC,
}

fn now_naive() -> NaiveDateTime {
    Local::now().naive_local()
}

fn ceil_seconds(d: Duration) -> i64 {
    let micros = d
        .num_microseconds()
        .unwrap_or_else(|| d.num_milliseconds().saturating_mul(1000));
    (micros as f64 / 1e6).ceil() as i64
}

fn duration_secs_f64(d: Duration) -> f64 {
    d.num_microseconds()
        .map(|u| u as f64 / 1e6)
        .unwrap_or_else(|| d.num_milliseconds() as f64 / 1e3)
}

// Waits for a mapped hotkey until the deadline (None = wait forever).
fn wait_input(deadline: Option<NaiveDateTime>) -> io::Result<Option<Input>> {
    loop {
        let timeout = match deadline {
            Some(d) => match (d - now_naive()).to_std() {
                Ok(t) => t,
                Err(_) => return Ok(None),
            },
            None => std::time::Duration::from_secs(86400),
        };
        if !event::poll(timeout)? {
            if deadline.is_some() {
                return Ok(None);
            }
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
            {
                return Ok(Some(Input::CtrlC));
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(Some(Input::Exit)),
                KeyCode::Char(' ') => return Ok(Some(Input::Pause)),
                KeyCode::Char('r') | KeyCode::Char('R') => return Ok(Some(Input::Reset)),
                KeyCode::Char('l') | KeyCode::Char('L') => return Ok(Some(Input::Lap)),
                KeyCode::Char('+') => return Ok(Some(Input::Plus)),
                KeyCode::Char('-') => return Ok(Some(Input::Minus)),
                _ => {}
            }
        }
    }
}

fn set_title(title: &str) {
    let mut out = io::stdout();
    let _ = write!(out, "\x1b]2;{}\x07", title);
    let _ = out.flush();
}

fn bell() {
    let mut out = io::stdout();
    let _ = write!(out, "\x07");
    let _ = out.flush();
}

fn write_outfile(cli: &Cli, line: &str, seconds: i64) {
    if let Some(path) = &cli.outfile {
        let _ = std::fs::write(path, format!("{}\n{}\n", line, seconds));
    }
}

fn spawn_shell(cmd_template: &str, seconds: i64, annunciation: &str) {
    let cmd = cmd_template
        .replace("{0}", &seconds.to_string())
        .replace("{1}", annunciation);
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn spawn_voice(voice_cmd: &str, voice: &str, annunciation: &str) {
    let _ = Command::new(voice_cmd)
        .args(["-v", voice, annunciation])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn find_voice_cmd() -> Option<&'static str> {
    ["/usr/bin/say", "/usr/bin/espeak", "/usr/bin/espeak-ng"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
}

fn render_title(cli: &Cli, font: &Font) -> Option<String> {
    cli.title.as_ref().map(|t| {
        if cli.no_figlet {
            t.clone()
        } else {
            font.render(t)
        }
    })
}

fn draw_frame(
    terminal: &mut DefaultTerminal,
    cli: &Cli,
    font: &Font,
    text_plain: &str,
    style: Style,
    title: Option<&str>,
    title_in_fallback: bool,
    y_offset: i64,
) -> io::Result<()> {
    let body = if cli.no_figlet {
        text_plain.to_string()
    } else {
        font.render(text_plain)
    };
    let fallback = match title {
        Some(t) if title_in_fallback => format!("{}\n{}", t, text_plain),
        _ => text_plain.to_string(),
    };
    terminal.draw(|f| draw::draw_text(f, &body, style, &fallback, title, y_offset))?;
    Ok(())
}

pub fn countdown(
    terminal: &mut DefaultTerminal,
    cli: &Cli,
    font: &Font,
    timespec: &str,
    parsed: (NaiveDateTime, NaiveDateTime),
    time_format: &str,
) -> io::Result<i32> {
    let (mut sync_start, mut target) = parsed;
    let mut text: Option<String> = cli.text.clone();
    let title_disp = render_title(cli, font);
    let y_offset = if cli.no_figlet {
        cli.no_figlet_y_offset
    } else {
        -1
    };
    let voice_cmd = if cli.voice.is_some() {
        find_voice_cmd()
    } else {
        None
    };
    let voice_prefix: String = if cli.voice.is_some() || cli.exec_cmd.is_some() {
        cli.voice_prefix.clone().unwrap_or_default()
    } else {
        String::new()
    };

    let seconds_total = ceil_seconds(target - now_naive());
    let mut seconds_left = seconds_total;

    'outer: while seconds_left > 0 || cli.blink || text.is_some() {
        let countdown_text = if cli.time {
            Local::now().format(time_format).to_string()
        } else if cli.alt_format {
            format_seconds_alt(seconds_left, seconds_total, cli.no_seconds)
        } else {
            format_seconds(seconds_left, cli.no_seconds)
        };
        if seconds_left > 0 {
            if !cli.no_window_title {
                set_title(&countdown_text);
            }
            write_outfile(cli, &countdown_text, seconds_left);
            let style = if seconds_left <= cli.critical {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            draw_frame(
                terminal,
                cli,
                font,
                &countdown_text,
                style,
                title_disp.as_deref(),
                true,
                y_offset,
            )?;
        }

        let annunciation: Option<String> = if seconds_left <= cli.critical {
            Some(seconds_left.to_string())
        } else if [5, 10, 20, 30, 60].contains(&seconds_left) {
            Some(format!("{} {} seconds", voice_prefix, seconds_left))
        } else if [300, 600, 1800].contains(&seconds_left) {
            Some(format!("{} {} minutes", voice_prefix, seconds_left / 60))
        } else if seconds_left == 3600 {
            Some(format!("{} one hour", voice_prefix))
        } else {
            None
        };
        if annunciation.is_some() || cli.exec_cmd.is_some() {
            if let Some(cmd) = &cli.exec_cmd {
                spawn_shell(cmd, seconds_left, annunciation.as_deref().unwrap_or(""));
            }
            if let (Some(vc), Some(ann)) = (voice_cmd, &annunciation) {
                spawn_voice(vc, cli.voice.as_deref().unwrap_or(""), ann.trim());
            }
        }

        let mut sleep_target = sync_start + Duration::seconds(1);
        if cli.time {
            sleep_target = sleep_target.with_nanosecond(0).unwrap();
        }
        let now = now_naive();
        if sleep_target > now && seconds_left > 0 {
            let mut action = wait_input(Some(sleep_target))?;
            if action == Some(Input::Pause) {
                let pause_start = now_naive();
                draw_frame(
                    terminal,
                    cli,
                    font,
                    &countdown_text,
                    Style::default().fg(Color::Green),
                    title_disp.as_deref(),
                    false,
                    y_offset,
                )?;
                let resumed = wait_input(None)?;
                if resumed == Some(Input::Pause) {
                    let time_paused = now_naive() - pause_start;
                    sync_start += time_paused;
                    target += time_paused;
                }
                action = resumed;
            }
            match action {
                Some(Input::Exit) => break 'outer,
                Some(Input::CtrlC) => return Ok(1),
                Some(Input::Reset) => {
                    let (s, t) = parse_timestr(timespec)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                    sync_start = s;
                    target = t;
                    seconds_left = ceil_seconds(target - now_naive());
                    continue 'outer;
                }
                Some(Input::Plus) => target += Duration::seconds(10),
                Some(Input::Minus) => target -= Duration::seconds(10),
                Some(Input::Lap) => continue 'outer,
                _ => {}
            }
        }
        sync_start = sleep_target;
        seconds_left = ceil_seconds(target - now_naive());

        if seconds_left <= 0 {
            if !cli.no_bell {
                bell();
            }
            if let Some(t) = &text {
                if !cli.no_text_magic {
                    text = Some(normalize_text(t));
                }
            }
            write_outfile(cli, text.as_deref().unwrap_or("DONE"), 0);
            let rendered_text: Option<String> = text.as_ref().map(|t| {
                if cli.no_figlet {
                    t.clone()
                } else {
                    font.render(t)
                }
            });

            if cli.blink || text.is_some() {
                let base_style = if cli.blink {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                };
                let red_bg = Style::default().bg(Color::Red);
                let mut flip = true;
                let mut slept: f64 = 0.0;
                let mut extra_sleep: f64 = 0.0;
                loop {
                    set_title(if flip { "/" } else { "\\" });
                    let style = if flip { base_style } else { red_bg };
                    let body = rendered_text.as_deref().unwrap_or("");
                    let fallback = text.as_deref().unwrap_or("");
                    terminal
                        .draw(|f| draw::draw_text(f, body, style, fallback, None, y_offset))?;
                    if cli.blink {
                        flip = !flip;
                    }
                    let sleep_start = Instant::now();
                    let deadline =
                        now_naive() + Duration::microseconds(((0.5 + extra_sleep) * 1e6) as i64);
                    let mut action = wait_input(Some(deadline))?;
                    extra_sleep = 0.0;
                    let this_slept = sleep_start.elapsed().as_secs_f64();
                    if action == Some(Input::Pause) {
                        let resumed = wait_input(None)?;
                        extra_sleep = this_slept;
                        action = resumed;
                    }
                    match action {
                        Some(Input::Exit) => return Ok(0),
                        Some(Input::CtrlC) => return Ok(1),
                        Some(Input::Reset) => {
                            let (s, t) = parse_timestr(timespec)
                                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                            sync_start = s;
                            target = t;
                            seconds_left = ceil_seconds(target - now_naive());
                            break;
                        }
                        _ => {}
                    }
                    slept += this_slept;
                    if let Some(quit_after) = cli.quit_after {
                        if slept >= quit_after {
                            return Ok(0);
                        }
                    }
                }
                // the blink/text loop only exits via a reset; restart the countdown
                continue 'outer;
            }
        }
    }
    Ok(0)
}

pub fn stopwatch(
    terminal: &mut DefaultTerminal,
    cli: &Cli,
    font: &Font,
    time_format: &str,
) -> io::Result<(f64, Vec<f64>, i32)> {
    let title_disp = render_title(cli, font);
    let y_offset = if cli.no_figlet {
        cli.no_figlet_y_offset
    } else {
        -1
    };
    let voice_prefix: String = cli.voice_prefix.clone().unwrap_or_default();

    let mut sync_start = now_naive();
    let mut pause_start: Option<NaiveDateTime> = None;
    let mut seconds_elapsed: i64 = 0;
    let mut laps: Vec<f64> = Vec::new();
    let mut exit_code = 0i32;

    loop {
        if let Some(quit_after) = cli.quit_after {
            if seconds_elapsed >= quit_after as i64 {
                break;
            }
        }
        let stopwatch_text = if cli.time {
            Local::now().format(time_format).to_string()
        } else if cli.alt_format {
            format_seconds_alt(seconds_elapsed, 0, cli.no_seconds)
        } else {
            format_seconds(seconds_elapsed, cli.no_seconds)
        };
        if !cli.no_window_title {
            set_title(&stopwatch_text);
        }
        write_outfile(cli, &stopwatch_text, seconds_elapsed);
        draw_frame(
            terminal,
            cli,
            font,
            &stopwatch_text,
            Style::default(),
            title_disp.as_deref(),
            true,
            y_offset,
        )?;

        if let Some(cmd) = &cli.exec_cmd {
            let annunciation = if seconds_elapsed <= cli.critical && seconds_elapsed > 0 {
                seconds_elapsed.to_string()
            } else if [5, 10, 20, 30, 40, 50, 60].contains(&seconds_elapsed) {
                format!("{} {} seconds", voice_prefix, seconds_elapsed)
            } else if [120, 180, 300, 600, 1800].contains(&seconds_elapsed) {
                format!("{} {} minutes", voice_prefix, seconds_elapsed / 60)
            } else if seconds_elapsed == 3600 {
                format!("{} one hour", voice_prefix)
            } else if seconds_elapsed > 0 && seconds_elapsed % 3600 == 0 {
                format!("{} {} hours", voice_prefix, seconds_elapsed / 3600)
            } else {
                String::new()
            };
            spawn_shell(cmd, seconds_elapsed, &annunciation);
        }

        let mut sleep_target = sync_start + Duration::seconds(seconds_elapsed + 1);
        if cli.time {
            sleep_target = sleep_target.with_nanosecond(0).unwrap();
        }
        let now = now_naive();
        if sleep_target > now {
            let mut action = wait_input(Some(sleep_target))?;
            if action == Some(Input::Pause) {
                pause_start = Some(now_naive());
                if !cli.no_window_title {
                    set_title(&stopwatch_text);
                }
                write_outfile(cli, &stopwatch_text, seconds_elapsed);
                draw_frame(
                    terminal,
                    cli,
                    font,
                    &stopwatch_text,
                    Style::default().fg(Color::Green),
                    title_disp.as_deref(),
                    false,
                    y_offset,
                )?;
                let resumed = wait_input(None)?;
                if resumed == Some(Input::Pause) {
                    sync_start += now_naive() - pause_start.unwrap();
                    pause_start = None;
                }
                action = resumed;
            }
            match action {
                Some(Input::Exit) | Some(Input::CtrlC) => {
                    if let Some(ps) = pause_start.take() {
                        sync_start += now_naive() - ps;
                    }
                    if action == Some(Input::CtrlC) {
                        exit_code = 1;
                    }
                    break;
                }
                Some(Input::Reset) => {
                    sync_start = now_naive();
                    laps.clear();
                }
                Some(Input::Plus) => sync_start -= Duration::seconds(10),
                Some(Input::Minus) => sync_start += Duration::seconds(10),
                Some(Input::Lap) => {
                    if let Some(ps) = pause_start.take() {
                        sync_start += now_naive() - ps;
                    }
                    laps.push(duration_secs_f64(now_naive() - sync_start));
                    sync_start = now_naive();
                }
                _ => {}
            }
        }
        seconds_elapsed = (now_naive() - sync_start).num_seconds();
    }
    let elapsed = duration_secs_f64(now_naive() - sync_start);
    Ok((elapsed, laps, exit_code))
}

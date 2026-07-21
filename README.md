# termdown (Rust)

A Rust/ratatui port of [termdown](https://github.com/trehn/termdown) 1.17.0,
the terminal countdown timer / stopwatch with big figlet digits. Written
against the Python reference source to be functionally equivalent: same CLI
flags, hotkeys, formatting, annunciation schedule, outfile format, and lap
output.

```
cargo build --release
./target/release/termdown 1h 5m      # countdown
./target/release/termdown            # stopwatch (laps printed to stderr on quit)
./target/release/termdown -z         # clock
```

Hotkeys: `SPACE` pause, `R` reset, `L` lap (stopwatch), `+`/`-` add/subtract
10 seconds, `Q` quit.

## Implementation notes

- The default font is `block`, a built-in 3x5 pixel font drawn with U+2588
  FULL BLOCK (two characters per pixel, so pixels stay roughly square).
  `-f FONT` selects a figlet font instead: `-f univers` gives the classic
  termdown look.
- The figlet renderer (`src/figlet.rs`) is a port of pyfiglet's loading and
  horizontal smushing engine, LTR only, without width-based line wrapping;
  where the Python version wraps or clips oversized output, this port falls
  back to plain text.
- `fonts/univers.flf` (the Python termdown default font) is vendored from
  pyfiglet and embedded. Other fonts are looked up as a literal path, then
  in `/usr/share/figlet`.
- `tests/data/univers_*.txt` are reference renderings generated with pyfiglet
  (`Figlet(font="univers", width=1000).renderText(...)`); the unit tests
  assert the Rust renderer matches them line for line.
- Absolute TIME values accept the documented forms (`12:00`, `2020-01-01`,
  `2020-01-01 14:00 UTC`) rather than the full dateutil grammar.
- `--no-text-magic` normalization covers common precomposed Latin accents via
  a table instead of full Unicode NFD.

# termdown (ratdown)

A Rust/ratatui port of termdown 1.17.0, the terminal countdown timer and stopwatch
with big figlet digits. Managed by the softorio hub:
`~/ws/softorio/projects/termdown.toml` (GitHub repo `jodavaho/ratdown`).

## Process

- How work is done: `~/ws/softorio/process/control.md`.
- Style: `~/ws/softorio/process/lang/rust.md`.

## Read first

- `README.md` (implementation notes)

## Commands

- Build: `cargo build --release`
- Test: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- No smoke: the binary is an interactive TUI.

## Rules

- The Python termdown 1.17.0 source is the reference: same CLI flags, hotkeys,
  formatting, annunciation schedule, outfile format and lap output. A deviation is a
  bug unless the README lists it.
- `tests/data/univers_*.txt` are pyfiglet reference renderings. Never edit them to
  make a test pass.
- `fonts/univers.flf` is vendored from pyfiglet; do not modify it.
- `.work/` belongs to softorio control.

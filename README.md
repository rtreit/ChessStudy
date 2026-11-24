# ChessStudy

ChessStudy is a read-only terminal browser for your Lichess studies and your recent completed Chess.com and Lichess games. Select a record to see its details and open the original study or game in your browser.

ChessStudy is an independent project and is not affiliated with Chess.com or Lichess.

## Setup

Use Rust 1.93 or newer and a terminal that supports UTF-8. Install the Rust `rustfmt` and `clippy` components for the development checks. Opening a record also requires a configured desktop browser.

From the repository root, run:

```sh
cp .env.example .env
```

Set these values in `.env`:

| Variable | Purpose |
| --- | --- |
| `CHESSCOM_USERNAME` | Your Chess.com username for the public game archive. |
| `LICHESS_USERNAME` | Your Lichess username for public studies and completed games. Optional when you use a token. |
| `LICHESS_TOKEN` | Optional [personal Lichess token](https://lichess.org/account/oauth/token) with `study:read` for studies visible to the signed-in account. If you also set a username, it must match the token account. |

Git ignores `.env`. Existing shell variables take precedence. Restart ChessStudy after changing the configuration.

```sh
cargo run --locked
```

Chess.com games open first. Both game views show up to 50 games, newest first. Chess.com games are ordered by completion time and Lichess games by start time. Studies are ordered by last update. Dates use UTC.

| Key | Action |
| --- | --- |
| `1`, `2`, `3` | Open Chess.com games, Lichess studies, or Lichess games |
| `Tab` / `Shift+Tab` or `←` / `→` | Switch views |
| `↑` / `↓`, `k` / `j`, `Page Up` / `Page Down`, `Home` / `End` | Move through rows or scroll details |
| `d` | Toggle the detail view in a narrow or short terminal |
| `Esc` | Return to the list from the detail view |
| `Enter` | Open the selected record in your browser |
| `r` | Reload the current view |
| `q` or `Ctrl+C` | Quit |

## Development

Application source and focused unit tests live in `src/`. Test API responses live in `tests/fixtures/`. The TUI renders text and has no runtime image assets. The contributor design skill and its assets live in `.agents/skills/`.

Run the same checks as CI from the repository root:

```sh
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The build and tests work without account credentials.

[Chess.com Published Data API](https://support.chess.com/en/articles/9650547-what-is-the-pubapi-and-how-do-i-use-it) · [Lichess API](https://lichess.org/api)

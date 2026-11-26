# ChessStudy

ChessStudy is a terminal browser for your Lichess studies and your recent completed Chess.com and Lichess games. Select a record to see its details, open the original, or import a game into a Lichess study.

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
| `LICHESS_TOKEN` | Optional [personal Lichess token](https://lichess.org/account/oauth/token) with `study:read` for studies visible to the signed-in account and `study:write` to create studies or import chapters. If you also set a username, it must match the token account. |

Git ignores `.env`. Existing shell variables take precedence. Restart ChessStudy after changing the configuration.

```sh
cargo run --locked
```

Chess.com games open first. Press `n` to load up to 50 older games from the archive. `Page Down` at the last loaded game also fetches the next batch. Reloading returns to the 50 most recent games. Chess.com games are ordered by completion time.

On the Lichess studies tab, press `n` to create a study. Enter a name and press Tab to choose private, unlisted, or public visibility; new studies start as private. Press Enter to create it.

To add a game to a study, select it and press `a`. Choose a study with the arrow keys and press Enter to import. The default clean import removes PGN comments and glyphs; press `c` to retain available comments and glyphs. Clean imports retain game tags, moves, and variations. Chapter names include the game date when available. The token account must be able to add chapters to the selected study. You can request computer analysis in Lichess after importing. If an import times out or its result is uncertain, check the study before retrying to avoid a duplicate.

The Lichess games view shows up to 50 games ordered by start time. Both game views label results from your account's perspective. Wins are green, losses are red, and draws are amber. Studies are ordered by last update. Select a study to see its numbered chapters in the detail pane. Dates use UTC.

| Key | Action |
| --- | --- |
| `1`, `2`, `3` | Open Chess.com games, Lichess studies, or Lichess games |
| `Tab` / `Shift+Tab` or `←` / `→` | Switch views |
| `↑` / `↓`, `k` / `j`, `Page Up` / `Page Down`, `Home` / `End` | Move through rows or scroll details |
| `n` | Load older Chess.com games, or create a study from the Lichess studies tab |
| `a` | Add the selected game to a Lichess study |
| `d` | Toggle the detail view in a narrow or short terminal |
| `[` / `]` | Scroll study details in a wide terminal |
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

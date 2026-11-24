use std::{
    io::{BufRead, BufReader},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Utc};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, RequestBuilder, Response},
    header::ACCEPT,
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::config::Config;

const RECENT_LIMIT: usize = 50;
const CHESSCOM: &str = "https://api.chess.com";
const LICHESS: &str = "https://lichess.org";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    ChessComGames,
    LichessStudies,
    LichessGames,
}

impl View {
    pub const ALL: [Self; 3] = [
        Self::ChessComGames,
        Self::LichessStudies,
        Self::LichessGames,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::ChessComGames => "Chess.com games",
            Self::LichessStudies => "Lichess studies",
            Self::LichessGames => "Lichess games",
        }
    }

    fn is_chesscom(self) -> bool {
        self == Self::ChessComGames
    }
}

pub struct Records {
    pub username: String,
    pub entries: Vec<Entry>,
}

pub struct Entry {
    pub url: Url,
    pub title: String,
    pub category: String,
    pub date: Option<DateTime<Utc>>,
    pub details: Vec<(&'static str, String)>,
}

pub type ResponseMessage = (View, Result<Records>);

pub struct Api {
    config: Config,
    client: Client,
    lichess_account: Option<String>,
    retry_after: [Option<Instant>; 2],
}

impl Api {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::builder()
                .user_agent(concat!(
                    "ChessStudy/",
                    env!("CARGO_PKG_VERSION"),
                    " (+https://github.com/rtreit/ChessStudy)"
                ))
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .context("Could not initialize the HTTP client")?,
            lichess_account: None,
            retry_after: [None; 2],
        })
    }

    pub fn spawn(mut self) -> (Sender<View>, Receiver<ResponseMessage>) {
        let (requests, pending) = mpsc::channel();
        let (completed, responses) = mpsc::channel();
        thread::spawn(move || {
            for view in pending {
                if completed.send((view, self.load(view))).is_err() {
                    break;
                }
            }
        });
        (requests, responses)
    }

    fn load(&mut self, view: View) -> Result<Records> {
        let mut records = match view {
            View::ChessComGames => self.chesscom_games()?,
            View::LichessStudies => self.lichess_studies()?,
            View::LichessGames => self.lichess_games()?,
        };
        records
            .entries
            .sort_by_key(|entry| std::cmp::Reverse(entry.date));
        if view != View::LichessStudies {
            records.entries.truncate(RECENT_LIMIT);
        }
        Ok(records)
    }

    fn send(&mut self, view: View, request: RequestBuilder) -> Result<Response> {
        let site = usize::from(view.is_chesscom());
        if let Some(remaining) =
            self.retry_after[site].and_then(|until| until.checked_duration_since(Instant::now()))
        {
            bail!(
                "Rate limited. Retry in {} seconds.",
                remaining.as_secs() + 1
            );
        }
        let response = request.send().map_err(|error| {
            if error.is_timeout() {
                anyhow::anyhow!("Request timed out. Press r to retry.")
            } else {
                anyhow::anyhow!("Could not reach the API. Check your connection and retry.")
            }
        })?;
        match response.status() {
            status if status.is_success() => Ok(response),
            StatusCode::TOO_MANY_REQUESTS => {
                let seconds = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(60)
                    .max(60);
                self.retry_after[site] = Some(Instant::now() + Duration::from_secs(seconds.into()));
                bail!("Rate limited. Retry in {seconds} seconds.");
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN if !view.is_chesscom() => {
                bail!("Lichess denied access. Check the token and its study:read scope.");
            }
            StatusCode::NOT_FOUND => bail!("The API returned 404. Check the account username."),
            status => bail!(
                "The API returned HTTP {}. Press r to retry.",
                status.as_u16()
            ),
        }
    }

    fn lichess_get(&self, path: &str) -> RequestBuilder {
        let request = self.client.get(format!("{LICHESS}{path}"));
        match &self.config.lichess_token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn lichess_username(&mut self, view: View) -> Result<String> {
        if let Some(name) = &self.lichess_account {
            return Ok(name.clone());
        }
        let name = if self.config.lichess_token.is_some() {
            #[derive(Deserialize)]
            struct Account {
                username: String,
            }
            let account: Account = read_json(self.send(view, self.lichess_get("/api/account"))?)?;
            if let Some(expected) = &self.config.lichess_username {
                ensure!(
                    account.username.eq_ignore_ascii_case(expected),
                    "LICHESS_USERNAME differs from the token account. Update .env and restart."
                );
            }
            ensure!(
                account
                    .username
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                    && !account.username.is_empty(),
                "Lichess returned an unexpected account username."
            );
            account.username
        } else {
            self.config
                .lichess_username
                .clone()
                .context("Set LICHESS_USERNAME or LICHESS_TOKEN in .env, then restart.")?
        };
        self.lichess_account = Some(name.clone());
        Ok(name)
    }

    fn lichess_studies(&mut self) -> Result<Records> {
        let view = View::LichessStudies;
        let username = self.lichess_username(view)?;
        let request = self
            .lichess_get(&format!("/api/study/by/{username}"))
            .header(ACCEPT, "application/x-ndjson");
        let studies: Vec<Study> = read_ndjson(self.send(view, request)?)?;
        let entries = studies
            .into_iter()
            .map(Study::into_entry)
            .collect::<Result<_>>()?;
        Ok(Records { username, entries })
    }

    fn lichess_games(&mut self) -> Result<Records> {
        let view = View::LichessGames;
        let username = self.lichess_username(view)?;
        let request = self
            .lichess_get(&format!("/api/games/user/{username}"))
            .query(&[
                ("max", RECENT_LIMIT.to_string()),
                ("moves", "false".into()),
                ("opening", "true".into()),
                ("ongoing", "false".into()),
            ])
            .header(ACCEPT, "application/x-ndjson");
        let games: Vec<LichessGame> = read_ndjson(self.send(view, request)?)?;
        let entries = games
            .into_iter()
            .map(LichessGame::into_entry)
            .collect::<Result<_>>()?;
        Ok(Records { username, entries })
    }

    fn chesscom_games(&mut self) -> Result<Records> {
        #[derive(Deserialize)]
        struct Archives {
            archives: Vec<String>,
        }
        #[derive(Deserialize)]
        struct Month {
            games: Vec<ChessComGame>,
        }

        let view = View::ChessComGames;
        let username = self
            .config
            .chesscom_username
            .clone()
            .context("Set CHESSCOM_USERNAME in .env, then restart.")?;
        let prefix = format!(
            "{CHESSCOM}/pub/player/{}/games/",
            username.to_ascii_lowercase()
        );
        let request = self.client.get(format!("{prefix}archives"));
        let mut archives: Archives = read_json(self.send(view, request)?)?;
        archives.archives.sort_unstable();
        archives.archives.dedup();
        let mut entries = Vec::new();
        for archive in archives.archives.into_iter().rev() {
            validate_archive(&archive, &prefix)?;
            let request = self.client.get(archive);
            let month: Month = read_json(self.send(view, request)?)?;
            for game in month.games {
                entries.push(game.into_entry()?);
            }
            if entries.len() >= RECENT_LIMIT {
                break;
            }
        }
        Ok(Records { username, entries })
    }
}

fn read_json<T: DeserializeOwned>(response: Response) -> Result<T> {
    response
        .json()
        .map_err(|_| anyhow::anyhow!("Could not read the API response. Press r to retry."))
}

fn read_ndjson<T: DeserializeOwned>(response: impl std::io::Read) -> Result<Vec<T>> {
    BufReader::new(response)
        .lines()
        .filter(|line| !matches!(line, Ok(text) if text.trim().is_empty()))
        .map(|line| {
            let line = line.map_err(|_| {
                anyhow::anyhow!("The API stream was interrupted. Press r to retry.")
            })?;
            serde_json::from_str(&line)
                .map_err(|_| anyhow::anyhow!("Could not read an API record. Press r to retry."))
        })
        .collect()
}

fn validate_archive(archive: &str, prefix: &str) -> Result<()> {
    let valid = archive.strip_prefix(prefix).is_some_and(|month| {
        month.len() == 7
            && chrono::NaiveDate::parse_from_str(&format!("{month}/01"), "%Y/%m/%d").is_ok()
    });
    ensure!(valid, "Chess.com returned an unexpected archive URL.");
    Ok(())
}

fn clean(text: impl AsRef<str>) -> String {
    text.as_ref()
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn date(date: Option<DateTime<Utc>>) -> String {
    date.map_or_else(
        || "Unknown".into(),
        |date| date.format("%Y-%m-%d %H:%M UTC").to_string(),
    )
}

fn site_link(base: &str, prefix: &str, id: &str) -> Result<Url> {
    ensure!(
        !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_alphanumeric()),
        "Lichess returned an unexpected record ID."
    );
    Ok(Url::parse(&format!("{base}/{prefix}{id}"))?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Study {
    id: String,
    name: String,
    created_at: i64,
    updated_at: i64,
}

impl Study {
    fn into_entry(self) -> Result<Entry> {
        let updated = DateTime::from_timestamp_millis(self.updated_at);
        Ok(Entry {
            url: site_link(LICHESS, "study/", &self.id)?,
            title: clean(self.name),
            category: "Study".into(),
            date: updated,
            details: vec![
                ("Updated", date(updated)),
                (
                    "Created",
                    date(DateTime::from_timestamp_millis(self.created_at)),
                ),
            ],
        })
    }
}

#[derive(Deserialize)]
struct LichessPlayer {
    user: Option<LichessUser>,
    rating: Option<i32>,
    #[serde(rename = "aiLevel")]
    ai_level: Option<u8>,
}

#[derive(Deserialize)]
struct LichessUser {
    name: String,
}

impl LichessPlayer {
    fn name(&self) -> String {
        if let Some(user) = &self.user {
            clean(&user.name)
        } else if let Some(level) = self.ai_level {
            format!("Stockfish level {level}")
        } else {
            "Anonymous".into()
        }
    }
}

#[derive(Deserialize)]
struct LichessPlayers {
    white: LichessPlayer,
    black: LichessPlayer,
}

#[derive(Deserialize)]
struct Opening {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LichessGame {
    id: String,
    players: LichessPlayers,
    speed: String,
    variant: String,
    created_at: i64,
    rated: bool,
    status: String,
    winner: Option<String>,
    opening: Option<Opening>,
}

impl LichessGame {
    fn into_entry(self) -> Result<Entry> {
        let date_played = DateTime::from_timestamp_millis(self.created_at);
        let result = match self.winner.as_deref() {
            Some("white") => "1–0",
            Some("black") => "0–1",
            _ if matches!(
                self.status.as_str(),
                "draw" | "stalemate" | "timeout" | "outoftime"
            ) =>
            {
                "½–½"
            }
            _ => "Unavailable",
        };
        let white = self.players.white.name();
        let black = self.players.black.name();
        Ok(Entry {
            url: site_link(LICHESS, "", &self.id)?,
            title: format!("{white} vs {black}"),
            category: clean(&self.speed),
            date: date_played,
            details: GameDetails {
                white: (&white, self.players.white.rating),
                black: (&black, self.players.black.rating),
                result,
                status: &self.status,
                variant: &self.variant,
                speed: &self.speed,
                rated: self.rated,
                date: date_played,
                date_label: "Started",
                opening: self.opening.map(|opening| clean(opening.name)),
            }
            .fields(),
        })
    }
}

#[derive(Deserialize)]
struct ChessComPlayer {
    username: String,
    rating: Option<i32>,
    result: String,
}

#[derive(Deserialize)]
struct ChessComGame {
    url: String,
    end_time: i64,
    time_class: String,
    rules: String,
    rated: bool,
    eco: Option<String>,
    white: ChessComPlayer,
    black: ChessComPlayer,
}

impl ChessComGame {
    fn into_entry(self) -> Result<Entry> {
        let url = Url::parse(&self.url)
            .ok()
            .filter(|url| {
                url.scheme() == "https"
                    && matches!(url.host_str(), Some("www.chess.com" | "chess.com"))
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.port().is_none()
            })
            .context("Chess.com returned an unexpected game URL.")?;
        let date_played = DateTime::from_timestamp(self.end_time, 0);
        let result = match (self.white.result.as_str(), self.black.result.as_str()) {
            ("win", _) => "1–0",
            (_, "win") => "0–1",
            (
                "agreed" | "repetition" | "stalemate" | "insufficient" | "50move"
                | "timevsinsufficient" | "draw",
                _,
            ) => "½–½",
            _ => "Unavailable",
        };
        let opening = self.eco.as_deref().and_then(|url| {
            let (_, slug) = url.split_once("/openings/")?;
            let slug = slug.trim_matches('/');
            if slug.is_empty() {
                return None;
            }
            Some(clean(slug.replace('-', " ")))
        });
        let white = clean(self.white.username);
        let black = clean(self.black.username);
        Ok(Entry {
            url,
            title: format!("{white} vs {black}"),
            category: clean(&self.time_class),
            date: date_played,
            details: GameDetails {
                white: (&white, self.white.rating),
                black: (&black, self.black.rating),
                result,
                status: &format!("{} / {}", self.white.result, self.black.result),
                variant: &self.rules,
                speed: &self.time_class,
                rated: self.rated,
                date: date_played,
                date_label: "Ended",
                opening,
            }
            .fields(),
        })
    }
}

struct GameDetails<'a> {
    white: (&'a str, Option<i32>),
    black: (&'a str, Option<i32>),
    result: &'a str,
    status: &'a str,
    variant: &'a str,
    speed: &'a str,
    rated: bool,
    date: Option<DateTime<Utc>>,
    date_label: &'static str,
    opening: Option<String>,
}

impl GameDetails<'_> {
    fn fields(self) -> Vec<(&'static str, String)> {
        fn player((name, rating): (&str, Option<i32>)) -> String {
            rating.map_or_else(|| name.into(), |rating| format!("{name} ({rating})"))
        }

        let mut details = vec![
            ("White", player(self.white)),
            ("Black", player(self.black)),
            ("Result", self.result.into()),
            ("Status", clean(self.status)),
            ("Variant", clean(self.variant)),
            ("Speed", clean(self.speed)),
            ("Rating", if self.rated { "Rated" } else { "Casual" }.into()),
            (self.date_label, date(self.date)),
        ];
        if let Some(opening) = self.opening {
            details.push(("Opening", opening));
        }
        details
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_study_from_a_stream_with_blank_lines() {
        let stream = include_bytes!("../tests/fixtures/lichess-studies.ndjson");
        let studies: Vec<Study> = read_ndjson(stream.as_slice()).unwrap();
        assert_eq!(studies.len(), 2);
        let first = studies.into_iter().next().unwrap().into_entry().unwrap();
        assert_eq!(first.url.as_str(), "https://lichess.org/study/study123");
        assert_eq!(first.title, "Rook endings");
        assert_eq!(first.details[0], ("Updated", "2025-11-02 00:00 UTC".into()));
    }

    #[test]
    fn labels_a_lichess_stalemate_and_engine_opponent() {
        let stream = include_bytes!("../tests/fixtures/lichess-engine-draw.ndjson");
        let mut games: Vec<LichessGame> = read_ndjson(stream.as_slice()).unwrap();
        let game = games.remove(0).into_entry().unwrap();
        assert_eq!(game.title, "StudyPlayer vs Stockfish level 3");
        assert!(game.details.contains(&("Result", "½–½".into())));
        assert!(
            game.details
                .contains(&("Opening", "French Defense: Exchange Variation".into()))
        );
    }

    #[test]
    fn reports_a_chesscom_win_from_black_perspective() {
        let game: ChessComGame =
            serde_json::from_str(include_str!("../tests/fixtures/chesscom-game.json")).unwrap();
        let entry = game.into_entry().unwrap();
        assert_eq!(entry.category, "rapid");
        assert!(entry.details.contains(&("Result", "0–1".into())));
        assert!(entry.details.contains(&("Black", "Opponent (1551)".into())));
        assert!(
            entry
                .details
                .contains(&("Opening", "French Defense Exchange Variation".into()))
        );
    }

    #[test]
    fn rejects_archive_urls_outside_the_configured_player_and_month() {
        let prefix = "https://api.chess.com/pub/player/studyplayer/games/";
        assert!(validate_archive(&format!("{prefix}2025/11"), prefix).is_ok());
        for suffix in ["2025/13", "2025/11?more", "../test"] {
            assert!(validate_archive(&format!("{prefix}{suffix}"), prefix).is_err());
        }
        assert!(
            validate_archive(
                "https://api.chess.com/pub/player/other/games/2025/11",
                prefix
            )
            .is_err()
        );
    }
}

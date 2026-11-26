use std::{
    collections::VecDeque,
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

const GAME_BATCH_SIZE: usize = 50;
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
    pub has_older: bool,
}

#[derive(Clone)]
pub struct Entry {
    pub url: Url,
    pub title: String,
    pub category: String,
    pub date: Option<DateTime<Utc>>,
    pub outcome: Option<GameOutcome>,
    pub details: Vec<(&'static str, String)>,
    pub pgn: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}

enum GameScore {
    WhiteWin,
    BlackWin,
    Draw,
    Unknown,
}

impl GameScore {
    fn label(&self) -> &'static str {
        match self {
            Self::WhiteWin => "1–0",
            Self::BlackWin => "0–1",
            Self::Draw => "½–½",
            Self::Unknown => "Unavailable",
        }
    }

    fn for_account(
        &self,
        white: Option<&str>,
        black: Option<&str>,
        username: &str,
    ) -> Option<GameOutcome> {
        let is_account =
            |name: Option<&str>| name.is_some_and(|name| name.eq_ignore_ascii_case(username));
        match (is_account(white), is_account(black), self) {
            (true, false, Self::WhiteWin) | (false, true, Self::BlackWin) => Some(GameOutcome::Win),
            (true, false, Self::BlackWin) | (false, true, Self::WhiteWin) => {
                Some(GameOutcome::Loss)
            }
            (true, false, Self::Draw) | (false, true, Self::Draw) => Some(GameOutcome::Draw),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Reload(View),
    OlderChessComGames,
}

impl Request {
    pub const fn view(self) -> View {
        match self {
            Self::Reload(view) => view,
            Self::OlderChessComGames => View::ChessComGames,
        }
    }
}

pub type ResponseMessage = (Request, Result<Records>);

pub struct ImportJob {
    pub game: Entry,
    pub source: View,
    pub study: Entry,
    pub annotated: bool,
}

pub enum WriteJob {
    Import(Box<ImportJob>),
    CreateStudy { name: String, visibility: String },
}

pub fn spawn_importer(config: Config) -> (Sender<WriteJob>, Receiver<Result<String>>) {
    let (sender, jobs) = mpsc::channel::<WriteJob>();
    let (completed, results) = mpsc::channel();
    thread::spawn(move || {
        let mut api = match Api::new(config) {
            Ok(api) => api,
            Err(_) => return,
        };
        for job in jobs {
            let result = match job {
                WriteJob::Import(job) => api.import_game(*job),
                WriteJob::CreateStudy { name, visibility } => api.create_study(&name, &visibility),
            };
            if completed.send(result).is_err() {
                break;
            }
        }
    });
    (sender, results)
}

pub type ChapterResult = (String, Result<Vec<String>>);

pub fn spawn_chapters(config: Config) -> (Sender<String>, Receiver<ChapterResult>) {
    let (sender, jobs) = mpsc::channel::<String>();
    let (completed, results) = mpsc::channel();
    thread::spawn(move || {
        let Ok(mut api) = Api::new(config) else {
            return;
        };
        for id in jobs {
            let result = api.study_chapters(&id);
            if completed.send((id, result)).is_err() {
                break;
            }
        }
    });
    (sender, results)
}

pub struct Api {
    config: Config,
    client: Client,
    lichess_account: Option<String>,
    chesscom_archive: Option<ChessComArchive>,
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
            chesscom_archive: None,
            retry_after: [None; 2],
        })
    }

    pub fn spawn(mut self) -> (Sender<Request>, Receiver<ResponseMessage>) {
        let (requests, pending) = mpsc::channel();
        let (completed, responses) = mpsc::channel();
        thread::spawn(move || {
            for request in pending {
                if completed.send((request, self.load(request))).is_err() {
                    break;
                }
            }
        });
        (requests, responses)
    }

    fn load(&mut self, request: Request) -> Result<Records> {
        let mut records = match request {
            Request::Reload(View::ChessComGames) => return self.reload_chesscom_games(),
            Request::OlderChessComGames => return self.older_chesscom_games(),
            Request::Reload(View::LichessStudies) => self.lichess_studies()?,
            Request::Reload(View::LichessGames) => self.lichess_games()?,
        };
        records
            .entries
            .sort_by_key(|entry| std::cmp::Reverse(entry.date));
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
                anyhow::anyhow!("Request timed out.")
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
                bail!(
                    "Lichess denied access. Check the token, permissions, and required study scopes."
                );
            }
            StatusCode::NOT_FOUND => bail!("The API returned 404. Check the account username."),
            status => bail!("The API returned HTTP {}.", status.as_u16()),
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

    fn create_study(&mut self, name: &str, visibility: &str) -> Result<String> {
        let token = self
            .config
            .lichess_token
            .clone()
            .context("Set LICHESS_TOKEN with study:write in .env, then restart.")?;
        ensure!(
            (2..=100).contains(&name.chars().count()),
            "Study name must contain 2 to 100 characters."
        );
        ensure!(
            matches!(visibility, "private" | "unlisted" | "public"),
            "Invalid study visibility."
        );
        self.lichess_username(View::LichessStudies)?;
        let request = self
            .client
            .post(format!("{LICHESS}/api/study"))
            .bearer_auth(token)
            .form(&[
                ("name", name),
                ("visibility", visibility),
                ("computer", "everyone"),
                ("explorer", "everyone"),
                ("cloneable", "owner"),
                ("shareable", "owner"),
                ("chat", "member"),
            ]);
        let response = self.send(View::LichessStudies, request).map_err(|error| anyhow::anyhow!("Creation failed or its result is unknown: {error}. Check your studies before retrying."))?;
        #[derive(Deserialize)]
        struct Created {
            id: String,
        }
        let created: Created = read_json(response)
            .context("Creation result is unknown. Check your studies before retrying.")?;
        site_link(LICHESS, "study/", &created.id)?;
        Ok(format!("Created {name}. Reloading studies."))
    }

    fn import_game(&mut self, job: ImportJob) -> Result<String> {
        let token = self
            .config
            .lichess_token
            .clone()
            .context("Set LICHESS_TOKEN with study:write in .env, then restart.")?;
        self.lichess_username(View::LichessStudies)?;
        let chapter_name = chapter_title(&job.game.title, job.game.date);
        let pgn = match job.source {
            View::ChessComGames => job
                .game
                .pgn
                .context("This Chess.com game has no PGN available.")?,
            View::LichessGames => {
                let id = job.game.url.path().trim_start_matches('/');
                ensure!(
                    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()),
                    "Invalid Lichess game ID."
                );
                let request = self
                    .lichess_get(&format!("/game/export/{id}"))
                    .query(&[("clocks", "true"), ("evals", "true"), ("opening", "true")])
                    .header(ACCEPT, "application/x-chess-pgn");
                self.send(View::LichessGames, request)?
                    .text()
                    .context("Could not read the game PGN.")?
            }
            View::LichessStudies => bail!("Select a game to import."),
        };
        ensure!(!pgn.trim().is_empty(), "The game PGN is empty.");
        let pgn = if job.annotated {
            pgn
        } else {
            strip_annotations(&pgn)
        };
        let study_id = job
            .study
            .url
            .path()
            .strip_prefix("/study/")
            .context("Invalid study URL.")?;
        ensure!(
            !study_id.is_empty() && study_id.bytes().all(|b| b.is_ascii_alphanumeric()),
            "Invalid study ID."
        );
        let request = self
            .client
            .post(format!("{LICHESS}/api/study/{study_id}/import-pgn"))
            .bearer_auth(token)
            .form(&[("pgn", pgn.as_str()), ("name", chapter_name.as_str())]);
        let response = self.send(View::LichessStudies, request).map_err(|error| {
            anyhow::anyhow!(
                "Import failed or its result is unknown: {error}. Check the study before retrying."
            )
        })?;
        #[derive(Deserialize)]
        struct Imported {
            chapters: Option<Vec<serde_json::Value>>,
            error: Option<String>,
        }
        let imported: Imported = read_json(response)
            .context("Import result is unknown. Check the study before retrying.")?;
        if let Some(error) = imported.error {
            bail!(
                "Lichess reported: {}. Check the study before retrying.",
                clean(error)
            );
        }
        ensure!(
            imported
                .chapters
                .as_ref()
                .is_some_and(|chapters| chapters.len() == 1),
            "Unexpected import result. Check the study before retrying."
        );
        Ok(format!("Added to {}", job.study.title))
    }

    fn study_chapters(&mut self, id: &str) -> Result<Vec<String>> {
        ensure!(
            id.len() == 8 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
            "Invalid study ID."
        );
        let request = self
            .lichess_get(&format!("/api/study/{id}.pgn"))
            .query(&[
                ("comments", "false"),
                ("variations", "false"),
                ("clocks", "false"),
            ])
            .header(ACCEPT, "application/x-chess-pgn");
        let pgn = self
            .send(View::LichessStudies, request)?
            .text()
            .context("Could not read study chapters.")?;
        chapter_names(&pgn)
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
        Ok(Records {
            username,
            entries,
            has_older: false,
        })
    }

    fn lichess_games(&mut self) -> Result<Records> {
        let view = View::LichessGames;
        let username = self.lichess_username(view)?;
        let request = self
            .lichess_get(&format!("/api/games/user/{username}"))
            .query(&[
                ("max", GAME_BATCH_SIZE.to_string()),
                ("moves", "false".into()),
                ("opening", "true".into()),
                ("ongoing", "false".into()),
            ])
            .header(ACCEPT, "application/x-ndjson");
        let games: Vec<LichessGame> = read_ndjson(self.send(view, request)?)?;
        let entries = games
            .into_iter()
            .map(|game| game.into_entry(&username))
            .collect::<Result<_>>()?;
        Ok(Records {
            username,
            entries,
            has_older: false,
        })
    }

    fn reload_chesscom_games(&mut self) -> Result<Records> {
        #[derive(Deserialize)]
        struct Archives {
            archives: Vec<String>,
        }

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
        let mut response: Archives = read_json(self.send(View::ChessComGames, request)?)?;
        for url in &response.archives {
            validate_archive(url, &prefix)?;
        }
        response.archives.sort_unstable();
        response.archives.dedup();

        let mut archive = ChessComArchive {
            username,
            months: response.archives,
            buffered: VecDeque::new(),
        };
        let records = archive.next_page(|month, username| self.chesscom_month(month, username))?;
        self.chesscom_archive = Some(archive);
        Ok(records)
    }

    fn older_chesscom_games(&mut self) -> Result<Records> {
        let mut archive = self
            .chesscom_archive
            .take()
            .context("Reload the Chess.com games first.")?;
        let result = archive.next_page(|month, username| self.chesscom_month(month, username));
        self.chesscom_archive = Some(archive);
        result
    }

    fn chesscom_month(&mut self, url: &str, username: &str) -> Result<Vec<Entry>> {
        #[derive(Deserialize)]
        struct Month {
            games: Vec<ChessComGame>,
        }

        let request = self.client.get(url);
        let month: Month = read_json(self.send(View::ChessComGames, request)?)?;
        month
            .games
            .into_iter()
            .map(|game| game.into_entry(username))
            .collect()
    }
}

struct ChessComArchive {
    username: String,
    // Archive URLs are ordered from oldest to newest, so the next month is at the end.
    months: Vec<String>,
    buffered: VecDeque<Entry>,
}

impl ChessComArchive {
    fn next_page(
        &mut self,
        mut load_month: impl FnMut(&str, &str) -> Result<Vec<Entry>>,
    ) -> Result<Records> {
        while self.buffered.len() < GAME_BATCH_SIZE {
            let Some(month) = self.months.last() else {
                break;
            };
            let mut games = load_month(month, &self.username)?;
            games.sort_by_key(|entry| std::cmp::Reverse(entry.date));
            self.buffered.extend(games);
            self.months.pop();
        }
        // Keep any games already fetched in the buffer if a later month fails.
        let entries = self
            .buffered
            .drain(..self.buffered.len().min(GAME_BATCH_SIZE))
            .collect();
        Ok(Records {
            username: self.username.clone(),
            entries,
            has_older: !self.buffered.is_empty() || !self.months.is_empty(),
        })
    }
}

// Remove PGN comments, NAGs and symbolic glyphs while keeping tags and variations.
fn strip_annotations(pgn: &str) -> String {
    let mut result = String::with_capacity(pgn.len());
    let mut chars = pgn.chars().peekable();
    let mut header = false;
    let mut quoted = false;
    let mut escaped = false;
    while let Some(c) = chars.next() {
        if header {
            result.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' && quoted {
                escaped = true;
            } else if c == '"' {
                quoted = !quoted;
            } else if c == ']' && !quoted {
                header = false;
            }
            continue;
        }
        match c {
            '[' => {
                header = true;
                result.push(c);
            }
            '{' => {
                for ch in chars.by_ref() {
                    if ch == '}' {
                        break;
                    }
                }
                result.push(' ');
            }
            ';' => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        result.push('\n');
                        break;
                    }
                }
            }
            '$' => {
                while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    chars.next();
                }
                result.push(' ');
            }
            '!' | '?' => {}
            _ => result.push(c),
        }
    }
    result
}

fn chapter_names(pgn: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for line in pgn.lines() {
        let Some(value) = line
            .trim()
            .strip_prefix("[ChapterName \"")
            .and_then(|s| s.strip_suffix("\"]"))
        else {
            continue;
        };
        let mut title = String::new();
        let mut escaped = false;
        for c in value.chars() {
            if escaped {
                title.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else {
                title.push(c);
            }
        }
        ensure!(!escaped, "Could not read chapter names.");
        names.push(clean(title));
    }
    ensure!(
        pgn.trim().is_empty() || !names.is_empty(),
        "Study export did not include chapter names."
    );
    Ok(names)
}

fn read_json<T: DeserializeOwned>(response: Response) -> Result<T> {
    response
        .json()
        .map_err(|_| anyhow::anyhow!("Could not read the API response."))
}

fn read_ndjson<T: DeserializeOwned>(response: impl std::io::Read) -> Result<Vec<T>> {
    BufReader::new(response)
        .lines()
        .filter(|line| !matches!(line, Ok(text) if text.trim().is_empty()))
        .map(|line| {
            let line = line.map_err(|_| anyhow::anyhow!("The API stream was interrupted."))?;
            serde_json::from_str(&line)
                .map_err(|_| anyhow::anyhow!("Could not read an API record."))
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

fn chapter_title(title: &str, date: Option<DateTime<Utc>>) -> String {
    let suffix = date
        .map(|date| format!(" · {}", date.format("%Y-%m-%d")))
        .unwrap_or_default();
    let available = 100_usize.saturating_sub(suffix.chars().count());
    let name: String = title.chars().take(available).collect();
    let name = name.trim();
    if name.is_empty() {
        format!("Game{suffix}")
    } else {
        format!("{name}{suffix}")
    }
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
            outcome: None,
            pgn: None,
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
    fn username(&self) -> Option<&str> {
        self.user.as_ref().map(|user| user.name.as_str())
    }

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
    fn into_entry(self, username: &str) -> Result<Entry> {
        let date_played = DateTime::from_timestamp_millis(self.created_at);
        let result = match self.winner.as_deref() {
            Some("white") => GameScore::WhiteWin,
            Some("black") => GameScore::BlackWin,
            None if matches!(
                self.status.as_str(),
                "draw" | "stalemate" | "timeout" | "outoftime"
            ) =>
            {
                GameScore::Draw
            }
            _ => GameScore::Unknown,
        };
        let white = self.players.white.name();
        let black = self.players.black.name();
        Ok(Entry {
            url: site_link(LICHESS, "", &self.id)?,
            title: format!("{white} vs {black}"),
            pgn: None,
            category: clean(&self.speed),
            date: date_played,
            outcome: result.for_account(
                self.players.white.username(),
                self.players.black.username(),
                username,
            ),
            details: GameDetails {
                white: (&white, self.players.white.rating),
                black: (&black, self.players.black.rating),
                result: result.label(),
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
    pgn: Option<String>,
    white: ChessComPlayer,
    black: ChessComPlayer,
}

impl ChessComGame {
    fn into_entry(self, username: &str) -> Result<Entry> {
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
        let white_result = chesscom_result(&self.white.result);
        let black_result = chesscom_result(&self.black.result);
        let result = match (white_result, black_result) {
            (Some(GameOutcome::Win), Some(GameOutcome::Win))
            | (Some(GameOutcome::Loss), Some(GameOutcome::Loss)) => GameScore::Unknown,
            (Some(GameOutcome::Win), _) => GameScore::WhiteWin,
            (_, Some(GameOutcome::Win)) => GameScore::BlackWin,
            (Some(GameOutcome::Draw), _) | (_, Some(GameOutcome::Draw)) => GameScore::Draw,
            (_, Some(GameOutcome::Loss)) => GameScore::WhiteWin,
            (Some(GameOutcome::Loss), _) => GameScore::BlackWin,
            _ => GameScore::Unknown,
        };
        let outcome = result.for_account(
            Some(&self.white.username),
            Some(&self.black.username),
            username,
        );
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
            pgn: self.pgn,
            category: clean(&self.time_class),
            date: date_played,
            outcome,
            details: GameDetails {
                white: (&white, self.white.rating),
                black: (&black, self.black.rating),
                result: result.label(),
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

fn chesscom_result(status: &str) -> Option<GameOutcome> {
    match status {
        "win" => Some(GameOutcome::Win),
        "agreed" | "repetition" | "stalemate" | "insufficient" | "50move"
        | "timevsinsufficient" | "draw" => Some(GameOutcome::Draw),
        "checkmated"
        | "timeout"
        | "resigned"
        | "lose"
        | "abandoned"
        | "kingofthehill"
        | "threecheck"
        | "bughousepartnerlose" => Some(GameOutcome::Loss),
        _ => None,
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
        let game = games.remove(0).into_entry("studyplayer").unwrap();
        assert_eq!(game.title, "StudyPlayer vs Stockfish level 3");
        assert_eq!(game.outcome, Some(GameOutcome::Draw));
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
        let entry = game.into_entry("opponent").unwrap();
        assert_eq!(entry.category, "rapid");
        assert_eq!(entry.outcome, Some(GameOutcome::Win));
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

    fn archive_games(first: i64, count: i64) -> Vec<Entry> {
        (first..first + count)
            .map(|id| Entry {
                url: Url::parse(&format!("https://www.chess.com/game/live/{id}")).unwrap(),
                title: id.to_string(),
                category: "rapid".into(),
                date: DateTime::from_timestamp(id, 0),
                outcome: None,
                pgn: None,
                details: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn chapter_title_includes_the_game_date_and_fits_the_import_limit() {
        let date = DateTime::parse_from_rfc3339("2026-11-24T12:34:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            chapter_title("Alice vs Bob", Some(date)),
            "Alice vs Bob · 2026-11-24"
        );
        assert_eq!(chapter_title("Alice vs Bob", None), "Alice vs Bob");
        let long_name = chapter_title(&"é".repeat(150), Some(date));
        assert_eq!(long_name.chars().count(), 100);
        assert!(long_name.ends_with("2026-11-24"));
    }

    #[test]
    fn extracts_ordered_chapter_names_and_unescapes_pgn_tags() {
        let pgn = "[Event \"Study\"]\n[ChapterName \"First\"]\n\n*\n\n[ChapterName \"A \\\"quoted\\\" line\"]\n\n1. e4 *";
        assert_eq!(
            chapter_names(pgn).unwrap(),
            vec!["First", "A \"quoted\" line"]
        );
        assert!(chapter_names("").unwrap().is_empty());
        assert!(chapter_names("[Event \"Missing\"]\n\n*").is_err());
    }

    #[test]
    fn clean_pgn_removes_comments_and_glyphs_but_preserves_tags_and_variations() {
        let pgn = "[Event \"What?\"]\n[White \"A!\"]\n\n1. e4! {[%clk 0:05:00] note} e5 $2 (1... c5?! {idea}) 2. Nf3 ;line note\nNc6 1-0";
        let cleaned = strip_annotations(pgn);
        assert!(cleaned.contains("[Event \"What?\"]"));
        assert!(cleaned.contains("[White \"A!\"]"));
        assert!(cleaned.contains("(1... c5 "));
        assert!(cleaned.contains("Nc6 1-0"));
        for annotation in ["[%clk", "note", "$2", "?!", "e4!"] {
            assert!(!cleaned.contains(annotation));
        }
    }

    #[test]
    fn pages_through_leftover_games_and_older_months_in_date_order() {
        let mut archive = ChessComArchive {
            username: "studyplayer".into(),
            months: vec!["oldest".into(), "empty".into(), "newest".into()],
            buffered: VecDeque::new(),
        };
        let mut calls = Vec::new();
        let mut load = |month: &str, _: &str| {
            calls.push(month.to_owned());
            Ok(match month {
                "newest" => archive_games(101, 55),
                "oldest" => archive_games(61, 40),
                "empty" => Vec::new(),
                _ => panic!("Unexpected month"),
            })
        };
        let first = archive.next_page(&mut load).unwrap();
        let second = archive.next_page(&mut load).unwrap();

        assert!(first.has_older);
        assert_eq!(first.entries.len(), 50);
        assert_eq!(second.entries.len(), 45);
        assert!(!second.has_older);
        assert_eq!(calls, ["newest", "empty", "oldest"]);
        let ids: Vec<i64> = first
            .entries
            .iter()
            .chain(&second.entries)
            .map(|entry| entry.date.unwrap().timestamp())
            .collect();
        assert_eq!(ids, (61..156).rev().collect::<Vec<_>>());
    }

    #[test]
    fn retries_the_failed_month_without_losing_the_games_already_fetched() {
        let mut archive = ChessComArchive {
            username: "studyplayer".into(),
            months: vec!["retry".into(), "loaded".into()],
            buffered: VecDeque::new(),
        };
        let first = archive.next_page(|month, _| match month {
            "loaded" => Ok(archive_games(61, 25)),
            "retry" => bail!("Temporary failure"),
            _ => panic!("Unexpected month"),
        });
        assert!(first.is_err());

        let resumed = archive
            .next_page(|month, _| {
                assert_eq!(month, "retry");
                Ok(archive_games(36, 25))
            })
            .unwrap();
        assert_eq!(resumed.entries.len(), 50);
        assert_eq!(resumed.entries[0].title, "85");
        assert_eq!(resumed.entries[49].title, "36");
        assert!(!resumed.has_older);
    }

    #[test]
    fn interprets_the_same_chesscom_game_as_a_loss_for_white() {
        let game: ChessComGame =
            serde_json::from_str(include_str!("../tests/fixtures/chesscom-game.json")).unwrap();
        let entry = game.into_entry("STUDYPLAYER").unwrap();
        assert_eq!(entry.outcome, Some(GameOutcome::Loss));
        assert!(entry.details.contains(&("Result", "0–1".into())));
    }

    #[test]
    fn treats_a_timeout_against_insufficient_material_as_a_draw() {
        let mut game: ChessComGame =
            serde_json::from_str(include_str!("../tests/fixtures/chesscom-game.json")).unwrap();
        game.white.result = "timeout".into();
        game.black.result = "timevsinsufficient".into();
        let entry = game.into_entry("studyplayer").unwrap();
        assert_eq!(entry.outcome, Some(GameOutcome::Draw));
        assert!(entry.details.contains(&("Result", "½–½".into())));
    }

    #[test]
    fn uses_the_lichess_players_to_assign_a_win_or_loss() {
        let fixture = include_bytes!("../tests/fixtures/lichess-engine-draw.ndjson");
        for (winner, expected) in [("white", GameOutcome::Win), ("black", GameOutcome::Loss)] {
            let mut game: LichessGame = read_ndjson(fixture.as_slice()).unwrap().remove(0);
            game.winner = Some(winner.into());
            game.status = "mate".into();
            assert_eq!(
                game.into_entry("studyplayer").unwrap().outcome,
                Some(expected)
            );
        }
        let game: LichessGame = read_ndjson(fixture.as_slice()).unwrap().remove(0);
        assert_eq!(game.into_entry("unknown-player").unwrap().outcome, None);
    }
}

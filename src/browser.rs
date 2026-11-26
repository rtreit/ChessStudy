use std::{
    collections::HashMap,
    io,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use crate::api::{
    ChapterResult, Entry, GameOutcome, ImportJob, Records, Request, ResponseMessage, View, WriteJob,
};

// A coordinated dark palette keeps semantic colors legible on both surfaces.
const CANVAS: Color = Color::Rgb(30, 30, 46);
const TEXT: Color = Color::Rgb(205, 214, 244);
const MUTED: Color = Color::Rgb(166, 173, 200);
const ACCENT: Color = Color::Rgb(137, 180, 250);
const SELECTION: Color = Color::Rgb(69, 71, 90);
const WARNING: Color = Color::Rgb(249, 226, 175);
const DETAIL_PANE_WIDTH: u16 = 90;

#[derive(Default)]
struct Page {
    records: Option<Records>,
    loading: Option<Request>,
    error: Option<(Request, String)>,
    selection: TableState,
}

impl Page {
    fn selected(&self) -> Option<&Entry> {
        self.records
            .as_ref()?
            .entries
            .get(self.selection.selected()?)
    }

    fn replace(&mut self, records: Records) {
        let index = self.selected().and_then(|previous| {
            records
                .entries
                .iter()
                .position(|entry| entry.url == previous.url)
        });
        let fallback = self.selection.selected().unwrap_or(0);
        let selected = index.or_else(|| {
            records
                .entries
                .len()
                .checked_sub(1)
                .map(|last| fallback.min(last))
        });
        self.selection.select(selected);
        self.records = Some(records);
    }

    fn has_older(&self) -> bool {
        self.records
            .as_ref()
            .is_some_and(|records| records.has_older)
    }

    fn at_end(&self) -> bool {
        self.records.as_ref().is_some_and(|records| {
            self.selection
                .selected()
                .is_some_and(|selected| selected + 1 == records.entries.len())
        })
    }

    fn append(&mut self, older: Records) {
        let advance = self.at_end() && !older.entries.is_empty();
        if let Some(records) = &mut self.records {
            let first_new = records.entries.len();
            records.entries.extend(older.entries);
            records.has_older = older.has_older;
            if advance {
                self.selection.select(Some(first_new));
            }
        }
    }

    fn step(&mut self, amount: isize) {
        if let Some(last) = self
            .records
            .as_ref()
            .and_then(|records| records.entries.len().checked_sub(1))
        {
            let current = self.selection.selected().unwrap_or(0);
            self.selection
                .select(Some(current.saturating_add_signed(amount).min(last)));
        }
    }
}

struct ImportDialog {
    game: Entry,
    source: View,
    selected: usize,
    annotated: bool,
    busy: bool,
    message: Option<String>,
    completed: bool,
}

struct CreateDialog {
    name: String,
    visibility: usize,
    busy: bool,
    completed: bool,
    message: Option<String>,
}

const VISIBILITIES: [&str; 3] = ["private", "unlisted", "public"];

struct Browser {
    view: View,
    pages: [Page; 3],
    requests: Sender<Request>,
    notice: Option<&'static str>,
    narrow_details: bool,
    detail_offset: usize,
    import: Option<ImportDialog>,
    imports: Sender<WriteJob>,
    create: Option<CreateDialog>,
    chapter_requests: Sender<String>,
    chapters: HashMap<String, Result<Vec<String>, String>>,
    chapter_pending: Option<(String, u64)>,
    chapter_generation: u64,
    chapter_selected: Option<String>,
}

impl Browser {
    fn page(&self) -> &Page {
        &self.pages[self.view.index()]
    }

    fn page_mut(&mut self) -> &mut Page {
        &mut self.pages[self.view.index()]
    }

    fn invalidate_chapters(&mut self) {
        self.chapters.clear();
        self.chapter_generation = self.chapter_generation.wrapping_add(1);
    }

    fn selected_study_id(&self) -> Option<String> {
        if self.view != View::LichessStudies {
            return None;
        }
        let path = self.page().selected()?.url.path();
        Some(path.strip_prefix("/study/")?.to_owned())
    }

    fn update_chapters(&mut self) {
        let selected = self.selected_study_id();
        if selected != self.chapter_selected {
            self.chapter_selected = selected.clone();
            self.detail_offset = 0;
        }
        let Some(id) = selected else {
            return;
        };
        if self.chapter_pending.is_none() && !self.chapters.contains_key(&id) {
            if self.chapter_requests.send(id.clone()).is_ok() {
                self.chapter_pending = Some((id, self.chapter_generation));
            } else {
                self.chapters.insert(
                    id,
                    Err("Chapter worker stopped. Restart ChessStudy.".into()),
                );
            }
        }
    }

    fn receive_chapters(&mut self, (id, result): ChapterResult) {
        if let Some((pending, generation)) = self.chapter_pending.take()
            && pending == id
            && generation == self.chapter_generation
        {
            self.chapters
                .insert(id, result.map_err(|error| error.to_string()));
        }
    }

    fn start_request(&mut self, request: Request) {
        if self.page().loading.is_some() {
            return;
        }
        let sent = self.requests.send(request).is_ok();
        let page = self.page_mut();
        page.loading = sent.then_some(request);
        page.error = (!sent).then(|| {
            (
                request,
                "The API worker stopped. Restart ChessStudy.".into(),
            )
        });
    }

    fn refresh(&mut self) {
        if self.view == View::LichessStudies {
            self.invalidate_chapters();
        }
        self.start_request(Request::Reload(self.view));
    }

    fn load_older(&mut self) {
        if self.page().has_older() {
            self.start_request(Request::OlderChessComGames);
        }
    }

    fn switch(&mut self, view: View) {
        self.view = view;
        self.narrow_details = false;
        self.detail_offset = 0;
        if self.page().records.is_none() && self.page().error.is_none() {
            self.refresh();
        }
    }

    fn receive(&mut self, (request, response): ResponseMessage) {
        let page = &mut self.pages[request.view().index()];
        page.loading = None;
        if response.is_ok() {
            page.error = None;
        }
        if request.view() == View::LichessStudies
            && let Ok(records) = &response
            && let Some(dialog) = &mut self.import
        {
            dialog.selected = dialog.selected.min(records.entries.len().saturating_sub(1));
        }
        match response {
            Ok(records) if request == Request::OlderChessComGames => page.append(records),
            Ok(records) => page.replace(records),
            Err(error) => page.error = Some((request, error.to_string())),
        }
    }

    fn key(&mut self, key: KeyEvent, width: u16, height: u16) -> bool {
        if key.kind == KeyEventKind::Release {
            return true;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        self.notice = None;
        if self.create.is_some() {
            self.create_key(key);
            return true;
        }
        if self.import.is_some() {
            self.import_key(key);
            return true;
        }
        let page_step = height.saturating_sub(8).max(1) as isize;
        let split = self.can_split(width, height);
        if !split
            && self.narrow_details
            && let Some(entry) = self.page().selected()
        {
            let movement = match key.code {
                KeyCode::Up | KeyCode::Char('k') => Some(-1),
                KeyCode::Down | KeyCode::Char('j') => Some(1),
                KeyCode::PageUp => Some(-page_step),
                KeyCode::PageDown => Some(page_step),
                KeyCode::Home => Some(isize::MIN),
                KeyCode::End => Some(isize::MAX),
                _ => None,
            };
            if let Some(amount) = movement {
                let max_offset = if self.view == View::LichessStudies {
                    entry.details.len()
                        + self
                            .chapters
                            .get(entry.url.path().trim_start_matches("/study/"))
                            .and_then(|result| result.as_ref().ok())
                            .map_or(1, Vec::len)
                        + 3
                } else {
                    entry.details.len() + 1
                };
                self.detail_offset = self
                    .detail_offset
                    .saturating_add_signed(amount)
                    .min(max_offset);
                return true;
            }
        }
        match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Char('[') if split && self.view == View::LichessStudies => {
                self.detail_offset = self.detail_offset.saturating_sub(1);
            }
            KeyCode::Char(']') if split && self.view == View::LichessStudies => {
                let max = self
                    .selected_study_id()
                    .and_then(|id| self.chapters.get(&id))
                    .and_then(|r| r.as_ref().ok())
                    .map_or(0, Vec::len);
                self.detail_offset = (self.detail_offset + 1).min(max);
            }
            KeyCode::Char('1') => self.switch(View::ChessComGames),
            KeyCode::Char('2') => self.switch(View::LichessStudies),
            KeyCode::Char('3') => self.switch(View::LichessGames),
            KeyCode::Tab | KeyCode::Right => self.switch(View::ALL[(self.view.index() + 1) % 3]),
            KeyCode::BackTab | KeyCode::Left => {
                self.switch(View::ALL[(self.view.index() + 2) % 3]);
            }
            KeyCode::Up | KeyCode::Char('k') => self.page_mut().step(-1),
            KeyCode::Down | KeyCode::Char('j') => self.page_mut().step(1),
            KeyCode::PageUp => self.page_mut().step(-page_step),
            KeyCode::PageDown if self.page().at_end() && self.page().has_older() => {
                self.load_older();
            }
            KeyCode::PageDown => self.page_mut().step(page_step),
            KeyCode::Char('n') if self.view == View::LichessStudies => {
                self.create = Some(CreateDialog {
                    name: String::new(),
                    visibility: 0,
                    busy: false,
                    completed: false,
                    message: None,
                });
            }
            KeyCode::Char('n') if split || !self.narrow_details => self.load_older(),
            KeyCode::Home => self.page_mut().step(isize::MIN),
            KeyCode::End => self.page_mut().step(isize::MAX),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('a')
                if self.view != View::LichessStudies && self.page().selected().is_some() =>
            {
                let game = self.page().selected().unwrap().clone();
                self.import = Some(ImportDialog {
                    game,
                    source: self.view,
                    selected: 0,
                    annotated: false,
                    busy: false,
                    message: None,
                    completed: false,
                });
                let studies = &self.pages[View::LichessStudies.index()];
                if studies.records.is_none() && studies.loading.is_none() {
                    let request = Request::Reload(View::LichessStudies);
                    let sent = self.requests.send(request).is_ok();
                    let studies = &mut self.pages[View::LichessStudies.index()];
                    studies.loading = sent.then_some(request);
                    if !sent {
                        studies.error = Some((
                            request,
                            "The API worker stopped. Restart ChessStudy.".into(),
                        ));
                    }
                }
            }
            KeyCode::Char('d') if !split && self.page().selected().is_some() => {
                self.narrow_details = !self.narrow_details;
                self.detail_offset = 0;
            }
            KeyCode::Esc => self.narrow_details = false,
            KeyCode::Enter => {
                if let Some(entry) = self.page().selected() {
                    self.notice = Some(if open::that_detached(entry.url.as_str()).is_ok() {
                        "Sent link to your browser."
                    } else {
                        "Could not launch a browser."
                    });
                }
            }
            _ => {}
        }
        true
    }

    fn create_key(&mut self, key: KeyEvent) {
        let dialog = self.create.as_mut().unwrap();
        if dialog.busy {
            return;
        }
        if key.code == KeyCode::Esc || (dialog.completed && key.code == KeyCode::Enter) {
            self.create = None;
            return;
        }
        if dialog.completed {
            return;
        }
        match key.code {
            KeyCode::Tab => dialog.visibility = (dialog.visibility + 1) % VISIBILITIES.len(),
            KeyCode::BackTab => {
                dialog.visibility =
                    (dialog.visibility + VISIBILITIES.len() - 1) % VISIBILITIES.len()
            }
            KeyCode::Backspace => {
                dialog.name.pop();
                dialog.message = None;
            }
            KeyCode::Char(c)
                if !c.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && dialog.name.chars().count() < 100 =>
            {
                dialog.name.push(c);
                dialog.message = None;
            }
            KeyCode::Enter => {
                let name = dialog.name.trim();
                if !(2..=100).contains(&name.chars().count()) {
                    dialog.message = Some("Enter a name of 2 to 100 characters.".into());
                } else if self
                    .imports
                    .send(WriteJob::CreateStudy {
                        name: name.to_owned(),
                        visibility: VISIBILITIES[dialog.visibility].to_owned(),
                    })
                    .is_ok()
                {
                    dialog.busy = true;
                    dialog.message = None;
                } else {
                    dialog.message = Some("Study worker stopped. Restart ChessStudy.".into());
                }
            }
            _ => {}
        }
    }

    fn draw_create(&self, frame: &mut Frame, area: Rect) {
        let dialog = self.create.as_ref().unwrap();
        let status = if dialog.busy {
            "Creating study…"
        } else {
            dialog.message.as_deref().unwrap_or("")
        };
        let shown_name: String = dialog
            .name
            .chars()
            .rev()
            .take(usize::from(area.width.saturating_sub(2)))
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let lines = vec![
            Line::styled("New Lichess study", Style::new().fg(ACCENT).bold()),
            Line::raw(""),
            Line::raw("Study name"),
            Line::styled(format!("{shown_name}▏"), Style::new().fg(TEXT)),
            Line::raw(""),
            Line::raw(format!("Visibility: {}", VISIBILITIES[dialog.visibility])),
            Line::styled(
                status,
                Style::new().fg(if dialog.completed { TEXT } else { WARNING }),
            ),
        ];
        let [body, footer] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(area);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), body);
        let controls = if dialog.busy {
            "Please wait…"
        } else if dialog.completed {
            "Enter / Esc Close"
        } else {
            "Tab Change visibility\nEnter Create  Esc Cancel"
        };
        frame.render_widget(
            Paragraph::new(controls).style(Style::new().fg(MUTED)),
            footer,
        );
    }

    fn import_key(&mut self, key: KeyEvent) {
        let dialog = self.import.as_mut().unwrap();
        if dialog.busy {
            return;
        }
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
            || (dialog.completed && key.code == KeyCode::Enter)
        {
            self.import = None;
            return;
        }
        if dialog.completed {
            return;
        }
        let studies = &self.pages[View::LichessStudies.index()];
        let entries = studies
            .records
            .as_ref()
            .map(|r| r.entries.as_slice())
            .unwrap_or(&[]);
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                dialog.selected = (dialog.selected + 1).min(entries.len().saturating_sub(1))
            }
            KeyCode::Up | KeyCode::Char('k') => dialog.selected = dialog.selected.saturating_sub(1),
            KeyCode::Char('c') | KeyCode::Char(' ') => dialog.annotated = !dialog.annotated,
            KeyCode::Char('r') if studies.loading.is_none() => {
                let request = Request::Reload(View::LichessStudies);
                if self.requests.send(request).is_ok() {
                    self.pages[View::LichessStudies.index()].loading = Some(request);
                }
            }
            KeyCode::Enter
                if studies.loading.is_none() && studies.error.is_none() && !entries.is_empty() =>
            {
                let study = entries[dialog.selected.min(entries.len() - 1)].clone();
                let job = ImportJob {
                    game: dialog.game.clone(),
                    source: dialog.source,
                    study,
                    annotated: dialog.annotated,
                };
                if self.imports.send(WriteJob::Import(Box::new(job))).is_ok() {
                    dialog.busy = true;
                    dialog.message = None;
                } else {
                    dialog.message = Some("Import worker stopped. Restart ChessStudy.".into());
                }
            }
            _ => {}
        }
    }

    fn draw_import(&self, frame: &mut Frame, area: Rect) {
        let dialog = self.import.as_ref().unwrap();
        let studies = &self.pages[View::LichessStudies.index()];
        let mut lines = vec![
            Line::styled("Add game to study", Style::new().fg(ACCENT).bold()),
            Line::raw(dialog.game.title.as_str()),
            Line::raw(if dialog.annotated {
                "Import: with comments and glyphs"
            } else {
                "Import: clean (no comments or glyphs)"
            }),
            Line::raw(""),
        ];
        if let Some(message) = &dialog.message {
            lines.push(Line::styled(
                message.as_str(),
                Style::new().fg(if dialog.completed { TEXT } else { WARNING }),
            ));
        } else if dialog.busy {
            lines.push(Line::raw("Importing…"));
        } else if studies.loading.is_some() {
            lines.push(Line::raw("Loading studies…"));
        } else if let Some((_, error)) = &studies.error {
            lines.push(Line::styled(error.as_str(), Style::new().fg(WARNING)));
            lines.push(Line::raw("r Retry"));
        } else if let Some(records) = &studies.records {
            if records.entries.is_empty() {
                lines.push(Line::raw(
                    "No studies found. Create a study on Lichess first.",
                ));
            } else {
                lines.push(Line::styled(
                    "Choose a study",
                    Style::new().fg(MUTED).bold(),
                ));
                let visible = usize::from(area.height.saturating_sub(8)).max(1);
                let offset = (dialog.selected + 1).saturating_sub(visible);
                for (index, entry) in records
                    .entries
                    .iter()
                    .enumerate()
                    .skip(offset)
                    .take(visible)
                {
                    let style = if index == dialog.selected {
                        Style::new().fg(TEXT).bg(SELECTION).bold()
                    } else {
                        Style::new()
                    };
                    lines.push(Line::styled(
                        format!(
                            "{} {}",
                            if index == dialog.selected { "›" } else { " " },
                            entry.title
                        ),
                        style,
                    ));
                }
            }
        }
        let [body, footer] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(area);
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), body);
        let controls = if dialog.busy {
            "Please wait…"
        } else if dialog.completed {
            "Enter / Esc Close"
        } else {
            "↑↓ Study  c Change mode\nEnter Import  r Retry  Esc Back"
        };
        frame.render_widget(
            Paragraph::new(controls).style(Style::new().fg(MUTED)),
            footer,
        );
    }

    fn status_height(&self, width: u16) -> u16 {
        let page = self.page();
        if page.error.is_some() && page.selected().is_some() {
            return if width < 48 { 5 } else { 4 };
        }
        2
    }

    fn can_split(&self, width: u16, height: u16) -> bool {
        width >= DETAIL_PANE_WIDTH && height >= 17 + self.status_height(width)
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Block::new().style(Style::new().fg(TEXT).bg(CANVAS)), area);
        if area.width < 32 || area.height < 12 {
            frame.render_widget(Paragraph::new("Resize to 32 × 12 or press q."), area);
            return;
        }
        if self.create.is_some() {
            self.draw_create(frame, area);
            return;
        }
        if self.import.is_some() {
            self.draw_import(frame, area);
            return;
        }
        let page = self.page();
        let stale_error = page.error.as_ref().filter(|_| page.selected().is_some());
        let status_height = self.status_height(area.width);
        let split = self.can_split(area.width, area.height);
        let [header, tabs, status, content, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(status_height),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new("ChessStudy").style(Style::new().fg(ACCENT).bold()),
            header,
        );
        let names = if area.width >= 57 {
            ["1 Chess.com games", "2 Lichess studies", "3 Lichess games"]
        } else {
            ["1 Chess.com", "2 Studies", "3 Lichess"]
        };
        let tab_gap = if area.width < 34 { " " } else { "  " };
        let tabs_line = View::ALL
            .into_iter()
            .zip(names)
            .flat_map(|(view, name)| {
                let style = if view == self.view {
                    Style::new()
                        .fg(ACCENT)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::new().fg(MUTED)
                };
                [Span::styled(name, style), Span::raw(tab_gap)]
            })
            .collect::<Line>();
        frame.render_widget(tabs_line, tabs);

        let summary = page.records.as_ref().map_or_else(
            || self.view.title().to_owned(),
            |records| {
                let count = records.entries.len();
                let item = match (self.view, count) {
                    (View::LichessStudies, 1) => "study",
                    (View::LichessStudies, _) => "studies",
                    (View::ChessComGames, 1) => "game loaded",
                    (View::ChessComGames, _) => "games loaded",
                    (_, 1) => "completed game",
                    (_, _) => "completed games",
                };
                let count = format!("{count} {item}");
                if self.view == View::ChessComGames
                    && records.username.len() + 5 + count.len() > usize::from(area.width)
                {
                    count
                } else {
                    format!("{}  ·  {count}", records.username)
                }
            },
        );
        let mut status_lines = vec![Line::raw(summary)];
        if let Some((request, error)) = stale_error {
            let warning = Style::new().fg(WARNING);
            let retry = match request {
                Request::OlderChessComGames => "Older games failed. Retry with n.",
                Request::Reload(_) => "Reload failed. Showing previous results.",
            };
            status_lines.push(Line::styled(retry, warning));
            status_lines.push(Line::styled(error, warning));
        } else if let Some(request) = page.loading {
            status_lines.push(Line::raw(match request {
                Request::OlderChessComGames => "Loading older games…",
                Request::Reload(_) if page.records.is_some() => "Refreshing…",
                Request::Reload(_) => "Loading…",
            }));
        } else if self.view == View::ChessComGames && page.selected().is_some() && !page.has_older()
        {
            status_lines.push(Line::raw("End of archive"));
        }
        frame.render_widget(
            Paragraph::new(status_lines)
                .style(Style::new().fg(MUTED))
                .wrap(Wrap { trim: true }),
            status,
        );

        if let Some(selected) = page.selected() {
            if split {
                let [list, details] =
                    Layout::horizontal([Constraint::Percentage(59), Constraint::Fill(1)])
                        .areas(content);
                let chapters = if self.view == View::LichessStudies {
                    self.selected_study_id()
                        .and_then(|id| self.chapters.get(&id))
                } else {
                    None
                };
                render_details(
                    frame,
                    details,
                    selected,
                    true,
                    self.detail_offset,
                    self.view == View::LichessStudies,
                    chapters,
                );
                render_entries(frame, list, self.view, self.page_mut());
            } else if self.narrow_details {
                let chapters = if self.view == View::LichessStudies {
                    self.selected_study_id()
                        .and_then(|id| self.chapters.get(&id))
                } else {
                    None
                };
                render_details(
                    frame,
                    content,
                    selected,
                    false,
                    self.detail_offset,
                    self.view == View::LichessStudies,
                    chapters,
                );
            } else {
                render_entries(frame, content, self.view, self.page_mut());
            }
        } else {
            let (text, style) = if let Some((_, error)) = &page.error {
                (error.as_str(), Style::new().fg(WARNING))
            } else if page.records.is_none() {
                ("", Style::new())
            } else if self.view == View::LichessStudies {
                ("No visible studies.", Style::new())
            } else {
                ("No completed games.", Style::new())
            };
            frame.render_widget(
                Paragraph::new(text).style(style).wrap(Wrap { trim: true }),
                content,
            );
        }

        let showing_details = !split && self.narrow_details;
        let movement = if showing_details { "Scroll" } else { "Move" };
        let older = self.page().has_older() && !showing_details;
        let controls = if split {
            let paging = if older { "  n Older games" } else { "" };
            let action = if self.view == View::LichessStudies {
                "n New study"
            } else {
                "a Add to study"
            };
            vec![
                Line::raw(format!(
                    "↑↓ {movement}  ↵ Open  {action}  r Reload  q Quit  Tab Next{paging}"
                )),
                Line::raw(self.notice.unwrap_or(if self.view == View::LichessStudies {
                    "[ ] Scroll chapters"
                } else {
                    ""
                })),
            ]
        } else {
            let action = if self.view == View::LichessStudies {
                "n New"
            } else {
                "a Add"
            };
            let main = format!("↑↓ {movement} ↵ Open {action} q Quit");
            let navigation = if older {
                "d Details  n Older  Tab Next"
            } else if showing_details {
                "d Back  Tab Next view"
            } else if self.page().selected().is_some() {
                "d Details  Tab Next view"
            } else {
                "Tab Next view"
            };
            vec![
                Line::raw(main),
                Line::raw(self.notice.unwrap_or(navigation)),
            ]
        };
        frame.render_widget(
            Paragraph::new(controls).style(Style::new().fg(MUTED)),
            footer,
        );
    }
}

fn render_entries(frame: &mut Frame, area: Rect, view: View, page: &mut Page) {
    let studies = view == View::LichessStudies;
    let show_speed = !studies && area.width >= 57;
    let show_date = studies || area.width >= 42;
    let mut columns = vec![Constraint::Fill(1)];
    let mut headings = vec![if studies { "Study" } else { "White vs Black" }];
    if !studies {
        columns.push(Constraint::Length(7));
        headings.push("Result");
    }
    if show_speed {
        columns.push(Constraint::Length(10));
        headings.push("Speed");
    }
    if show_date {
        columns.push(Constraint::Length(10));
        headings.push(if studies { "Updated" } else { "Date" });
    }
    let entries = &page.records.as_ref().unwrap().entries;
    let visible = usize::from(area.height.saturating_sub(1)).max(1);
    let selected = page.selection.selected().unwrap_or(0);
    let offset = page
        .selection
        .offset()
        .min(selected)
        .min(entries.len().saturating_sub(visible))
        .max((selected + 1).saturating_sub(visible));
    *page.selection.offset_mut() = offset;
    let mut visible_selection = TableState::new().with_selected(Some(selected - offset));
    let selected_style = Style::new().fg(TEXT).bg(SELECTION).bold();
    let rows = entries
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(index, entry)| {
            let mut cells = vec![Cell::new(entry.title.as_str())];
            if !studies {
                cells.push(Cell::new(outcome_label(entry.outcome)));
            }
            if show_speed {
                cells.push(Cell::new(entry.category.as_str()));
            }
            if show_date {
                cells.push(Cell::new(entry.date.map_or_else(
                    || "Unknown".into(),
                    |date| date.format("%Y-%m-%d").to_string(),
                )));
            }
            let row = Row::new(cells);
            if index == selected {
                row.style(selected_style)
            } else {
                row
            }
        });
    let table = Table::new(rows, columns)
        .header(Row::new(headings).style(Style::new().fg(MUTED).bold()))
        .column_spacing(2)
        .highlight_symbol("› ");
    frame.render_stateful_widget(table, area, &mut visible_selection);
}

fn outcome_label(outcome: Option<GameOutcome>) -> Span<'static> {
    let (label, foreground) = match outcome {
        Some(GameOutcome::Win) => ("Win", Color::Rgb(166, 227, 161)),
        Some(GameOutcome::Loss) => ("Loss", Color::Rgb(243, 139, 168)),
        Some(GameOutcome::Draw) => ("Draw", Color::Rgb(249, 226, 175)),
        None => return Span::raw("Unknown"),
    };
    Span::styled(label, Style::new().fg(foreground).bold())
}

fn render_details<'a>(
    frame: &mut Frame,
    area: Rect,
    entry: &'a Entry,
    bordered: bool,
    skip: usize,
    is_study: bool,
    chapters: Option<&'a Result<Vec<String>, String>>,
) {
    let mut lines = vec![
        Line::styled(entry.title.as_str(), Style::new().bold()),
        Line::raw(""),
    ];
    lines.extend(entry.details.iter().map(|(label, value)| {
        let label_span = Span::styled(format!("{label: <8}"), Style::new().fg(MUTED).bold());
        if *label == "Result" && entry.outcome.is_some() {
            Line::from(vec![
                label_span,
                outcome_label(entry.outcome),
                Span::raw(format!(" ({value})")),
            ])
        } else {
            Line::from(vec![label_span, Span::raw(value.as_str())])
        }
    }));
    if is_study {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Chapters", Style::new().fg(ACCENT).bold()));
        match chapters {
            None => lines.push(Line::raw("Loading chapters…")),
            Some(Err(error)) => {
                lines.push(Line::styled(error.as_str(), Style::new().fg(WARNING)));
                lines.push(Line::raw("Press r to retry."));
            }
            Some(Ok(names)) if names.is_empty() => lines.push(Line::raw("No chapters.")),
            Some(Ok(names)) => lines.extend(
                names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| Line::raw(format!("{}. {name}", i + 1))),
            ),
        }
    }
    let details =
        Paragraph::new(lines.into_iter().skip(skip).collect::<Vec<_>>()).wrap(Wrap { trim: true });
    let details = if bordered {
        details.block(
            Block::new()
                .borders(Borders::LEFT)
                .border_style(Style::new().fg(SELECTION))
                .padding(ratatui::widgets::Padding::left(2)),
        )
    } else {
        details
    };
    frame.render_widget(details, area);
}

pub fn run(
    terminal: &mut DefaultTerminal,
    requests: Sender<Request>,
    responses: Receiver<ResponseMessage>,
    imports: Sender<WriteJob>,
    import_results: Receiver<anyhow::Result<String>>,
    chapter_requests: Sender<String>,
    chapter_results: Receiver<ChapterResult>,
) -> io::Result<()> {
    let mut browser = Browser {
        view: View::ChessComGames,
        pages: std::array::from_fn(|_| Page::default()),
        requests,
        notice: None,
        narrow_details: false,
        detail_offset: 0,
        import: None,
        imports,
        create: None,
        chapter_requests,
        chapters: HashMap::new(),
        chapter_pending: None,
        chapter_generation: 0,
        chapter_selected: None,
    };
    browser.refresh();
    loop {
        while let Ok(message) = responses.try_recv() {
            browser.receive(message);
        }
        while let Ok(result) = chapter_results.try_recv() {
            browser.receive_chapters(result);
        }
        while let Ok(result) = import_results.try_recv() {
            if let Some(dialog) = browser.create.as_mut() {
                dialog.busy = false;
                dialog.completed = result.is_ok();
                dialog.message = Some(match result {
                    Ok(message) => message,
                    Err(error) => error.to_string(),
                });
                if dialog.completed {
                    browser.refresh();
                }
            } else if let Some(dialog) = browser.import.as_mut() {
                dialog.busy = false;
                dialog.completed = result.is_ok();
                dialog.message = Some(match result {
                    Ok(message) => message,
                    Err(error) => error.to_string(),
                });
                if dialog.completed {
                    browser.invalidate_chapters();
                }
            }
        }
        browser.update_chapters();
        terminal.draw(|frame| browser.draw(frame))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            let size = terminal.size()?;
            if !browser.key(key, size.width, size.height) {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

    fn records(titles: &[&str]) -> Records {
        let entries = titles
            .iter()
            .map(|title| Entry {
                url: Url::parse(&format!("https://lichess.org/study/{title}")).unwrap(),
                title: (*title).into(),
                category: "Study".into(),
                date: None,
                outcome: None,
                pgn: None,
                details: Vec::new(),
            })
            .collect();
        Records {
            username: "test-player".into(),
            entries,
            has_older: false,
        }
    }

    #[test]
    fn keeps_the_same_study_selected_when_a_new_study_appears_above_it() {
        let mut page = Page::default();
        page.replace(records(&["First", "Selected"]));
        page.step(1);
        page.replace(records(&["New", "First", "Selected"]));
        assert_eq!(
            page.selected().map(|entry| entry.title.as_str()),
            Some("Selected")
        );
    }

    #[test]
    fn clamps_selection_after_a_refresh_deletes_rows() {
        let mut page = Page::default();
        page.replace(records(&["First", "Removed"]));
        page.step(isize::MAX);
        page.replace(records(&["First"]));
        assert_eq!(
            page.selected().map(|entry| entry.title.as_str()),
            Some("First")
        );
        page.replace(records(&[]));
        assert!(page.selected().is_none());
    }

    #[test]
    fn keeps_results_available_when_refresh_fails() {
        let (requests, pending) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::LichessStudies,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: false,
            detail_offset: 0,
            import: None,
            create: None,
            chapter_requests: std::sync::mpsc::channel().0,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
            imports: std::sync::mpsc::channel().0,
        };
        browser.receive((
            Request::Reload(View::LichessStudies),
            Ok(records(&["First"])),
        ));
        browser.refresh();
        browser.refresh();
        assert!(matches!(
            pending.try_recv(),
            Ok(Request::Reload(View::LichessStudies))
        ));
        assert!(pending.try_recv().is_err());

        browser.receive((
            Request::Reload(View::LichessStudies),
            Err(anyhow::anyhow!("Could not connect")),
        ));
        assert!(browser.page().selected().is_some());
        assert_eq!(
            browser
                .page()
                .error
                .as_ref()
                .map(|(_, error)| error.as_str()),
            Some("Could not connect")
        );
        assert!(browser.page().loading.is_none());
    }

    #[test]
    fn keeps_all_tabs_and_quit_visible_in_the_smallest_detail_view() {
        use ratatui::{Terminal, backend::TestBackend};

        let (requests, _) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::LichessStudies,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: true,
            detail_offset: 0,
            import: None,
            create: None,
            chapter_requests: std::sync::mpsc::channel().0,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
            imports: std::sync::mpsc::channel().0,
        };
        browser.receive((
            Request::Reload(View::LichessStudies),
            Ok(records(&["First"])),
        ));
        let mut terminal = Terminal::new(TestBackend::new(32, 12)).unwrap();
        terminal.draw(|frame| browser.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer();
        let line = |row| {
            (0..32)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        };
        assert!(line(1).contains("1 Chess.com 2 Studies 3 Lichess"));
        assert!(line(10).contains("q Quit"));
        assert!(line(11).contains("d Back"));
    }

    #[test]
    fn page_down_at_the_end_fetches_older_games_and_moves_to_the_first_new_row() {
        let (requests, pending) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::ChessComGames,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: false,
            detail_offset: 0,
            import: None,
            create: None,
            chapter_requests: std::sync::mpsc::channel().0,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
            imports: std::sync::mpsc::channel().0,
        };
        let mut first = records(&["Newest", "LastLoaded"]);
        first.has_older = true;
        browser.receive((Request::Reload(View::ChessComGames), Ok(first)));

        let page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        browser.key(page_down, 100, 20);
        assert!(pending.try_recv().is_err());
        browser.key(page_down, 100, 20);
        assert!(matches!(
            pending.try_recv(),
            Ok(Request::OlderChessComGames)
        ));
        browser.key(page_down, 100, 20);
        assert!(pending.try_recv().is_err());

        browser.receive((
            Request::OlderChessComGames,
            Ok(records(&["Older", "Oldest"])),
        ));
        assert_eq!(browser.page().selected().unwrap().title, "Older");
        assert_eq!(browser.page().records.as_ref().unwrap().entries.len(), 4);
        browser.key(page_down, 100, 20);
        browser.key(page_down, 100, 20);
        assert!(pending.try_recv().is_err());
    }

    #[test]
    fn creates_a_named_study_with_explicit_visibility_and_no_duplicate_submission() {
        let (requests, _) = std::sync::mpsc::channel();
        let (imports, jobs) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::LichessStudies,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: false,
            detail_offset: 0,
            import: None,
            create: None,
            chapter_requests: std::sync::mpsc::channel().0,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
            imports,
        };
        browser.pages[1].replace(records(&[]));
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        browser.key(key(KeyCode::Char('a')), 32, 12);
        assert!(browser.import.is_none());
        browser.key(key(KeyCode::Char('n')), 32, 12);
        browser.key(key(KeyCode::Enter), 32, 12);
        assert!(jobs.try_recv().is_err());
        for c in "French Defense".chars() {
            browser.key(key(KeyCode::Char(c)), 32, 12);
        }
        assert_eq!(
            VISIBILITIES[browser.create.as_ref().unwrap().visibility],
            "private"
        );
        browser.key(key(KeyCode::Tab), 32, 12);
        browser.key(key(KeyCode::Enter), 32, 12);
        match jobs.try_recv().unwrap() {
            WriteJob::CreateStudy { name, visibility } => {
                assert_eq!(name, "French Defense");
                assert_eq!(visibility, "unlisted");
            }
            _ => panic!("Expected study creation"),
        }
        browser.key(key(KeyCode::Enter), 32, 12);
        assert!(jobs.try_recv().is_err());
    }

    #[test]
    fn study_creation_and_hints_fit_small_and_wide_terminals() {
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(32, 12), (120, 30)] {
            let (requests, _) = std::sync::mpsc::channel();
            let (imports, _) = std::sync::mpsc::channel();
            let mut browser = Browser {
                view: View::LichessStudies,
                pages: std::array::from_fn(|_| Page::default()),
                requests,
                notice: None,
                narrow_details: false,
                detail_offset: 0,
                import: None,
                create: None,
                chapter_requests: std::sync::mpsc::channel().0,
                chapters: HashMap::new(),
                chapter_pending: None,
                chapter_generation: 0,
                chapter_selected: None,
                imports,
            };
            browser.pages[1].replace(records(&["Study"]));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| browser.draw(frame)).unwrap();
            let read = |terminal: &Terminal<TestBackend>| {
                let buffer = terminal.backend().buffer();
                (0..height)
                    .map(|row| {
                        (0..width)
                            .map(|col| buffer[(col, row)].symbol())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let text = read(&terminal);
            assert!(text.contains("n New"));
            assert!(!text.contains("a Add"));
            browser.key(
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
                width,
                height,
            );
            terminal.draw(|frame| browser.draw(frame)).unwrap();
            let text = read(&terminal);
            assert!(text.contains("Visibility: private"));
            assert!(text.contains("Enter Create"));
            assert!(text.contains("Esc Cancel"));
        }
    }

    #[test]
    fn import_defaults_to_clean_and_sends_selected_study_and_mode() {
        let (requests, _) = std::sync::mpsc::channel();
        let (imports, jobs) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::ChessComGames,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: false,
            detail_offset: 0,
            import: None,
            create: None,
            chapter_requests: std::sync::mpsc::channel().0,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
            imports,
        };
        browser.pages[0].replace(records(&["Game"]));
        browser.pages[1].replace(records(&["First", "Second"]));
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        browser.key(key(KeyCode::Char('a')), 100, 24);
        assert!(!browser.import.as_ref().unwrap().annotated);
        browser.key(key(KeyCode::Down), 100, 24);
        browser.key(key(KeyCode::Char('c')), 100, 24);
        browser.key(key(KeyCode::Enter), 100, 24);
        let WriteJob::Import(job) = jobs.try_recv().unwrap() else {
            panic!("Expected import");
        };
        assert_eq!(job.study.title, "Second");
        assert!(job.annotated);
        assert!(browser.import.as_ref().unwrap().busy);
        browser.key(key(KeyCode::Enter), 100, 24);
        assert!(jobs.try_recv().is_err());
    }

    #[test]
    fn import_dialog_shows_mode_selection_and_controls_at_small_and_wide_sizes() {
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(32, 12), (120, 30)] {
            let (requests, _) = std::sync::mpsc::channel();
            let (imports, _) = std::sync::mpsc::channel();
            let mut browser = Browser {
                view: View::ChessComGames,
                pages: std::array::from_fn(|_| Page::default()),
                requests,
                notice: None,
                narrow_details: false,
                detail_offset: 0,
                import: None,
                create: None,
                chapter_requests: std::sync::mpsc::channel().0,
                chapters: HashMap::new(),
                chapter_pending: None,
                chapter_generation: 0,
                chapter_selected: None,
                imports,
            };
            browser.pages[0].replace(records(&["Game"]));
            browser.pages[1].replace(records(&["My study"]));
            browser.key(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
                width,
                height,
            );
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| browser.draw(frame)).unwrap();
            let buffer = terminal.backend().buffer();
            let text = (0..height)
                .map(|row| {
                    (0..width)
                        .map(|col| buffer[(col, row)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("Import: clean"));
            assert!(text.contains("My study"));
            assert!(text.contains("Esc Back"));
        }
    }

    #[test]
    fn study_chapters_follow_selection_and_discard_stale_responses() {
        let (requests, _) = std::sync::mpsc::channel();
        let (chapter_requests, pending) = std::sync::mpsc::channel();
        let mut browser = Browser {
            view: View::LichessStudies,
            pages: std::array::from_fn(|_| Page::default()),
            requests,
            notice: None,
            narrow_details: false,
            detail_offset: 0,
            import: None,
            create: None,
            imports: std::sync::mpsc::channel().0,
            chapter_requests,
            chapters: HashMap::new(),
            chapter_pending: None,
            chapter_generation: 0,
            chapter_selected: None,
        };
        browser.page_mut().replace(records(&["First", "Second"]));
        browser.update_chapters();
        assert_eq!(pending.try_recv().unwrap(), "First");
        browser.page_mut().step(1);
        browser.update_chapters();
        assert!(pending.try_recv().is_err());
        browser.invalidate_chapters();
        browser.receive_chapters(("First".into(), Ok(vec!["Old".into()])));
        assert!(!browser.chapters.contains_key("First"));
        browser.update_chapters();
        assert_eq!(pending.try_recv().unwrap(), "Second");
        browser.receive_chapters((
            "Second".into(),
            Ok(vec!["Opening".into(), "Endgame".into()]),
        ));
        assert_eq!(browser.chapters["Second"].as_ref().unwrap()[1], "Endgame");
    }

    #[test]
    fn study_details_render_numbered_chapters_and_scroll_in_wide_and_narrow_views() {
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(32, 12), (120, 30)] {
            let (requests, _) = std::sync::mpsc::channel();
            let mut browser = Browser {
                view: View::LichessStudies,
                pages: std::array::from_fn(|_| Page::default()),
                requests,
                notice: None,
                narrow_details: width < 90,
                detail_offset: 0,
                import: None,
                create: None,
                imports: std::sync::mpsc::channel().0,
                chapter_requests: std::sync::mpsc::channel().0,
                chapters: HashMap::new(),
                chapter_pending: None,
                chapter_generation: 0,
                chapter_selected: Some("First".into()),
            };
            browser.page_mut().replace(records(&["First"]));
            browser.chapters.insert(
                "First".into(),
                Ok((1..=40).map(|i| format!("Chapter {i}")).collect()),
            );
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| browser.draw(frame)).unwrap();
            let text = |terminal: &Terminal<TestBackend>| {
                let buffer = terminal.backend().buffer();
                (0..height)
                    .map(|row| {
                        (0..width)
                            .map(|col| buffer[(col, row)].symbol())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert!(text(&terminal).contains("1. Chapter 1"));
            let key = if width < 90 {
                KeyCode::End
            } else {
                KeyCode::Char(']')
            };
            for _ in 0..40 {
                browser.key(KeyEvent::new(key, KeyModifiers::NONE), width, height);
            }
            terminal.draw(|frame| browser.draw(frame)).unwrap();
            assert!(text(&terminal).contains("40. Chapter 40"));
        }
    }

    #[test]
    fn keeps_outcome_colors_visible_on_selected_and_unselected_narrow_rows() {
        use ratatui::{Terminal, backend::TestBackend};

        let mut page = Page::default();
        let mut games = records(&["First", "Second", "Third", "Fourth"]);
        for (entry, outcome) in games.entries.iter_mut().zip([
            Some(GameOutcome::Win),
            Some(GameOutcome::Loss),
            Some(GameOutcome::Draw),
            None,
        ]) {
            entry.outcome = outcome;
        }
        page.replace(games);
        let mut terminal = Terminal::new(TestBackend::new(32, 12)).unwrap();
        terminal
            .draw(|frame| render_entries(frame, frame.area(), View::ChessComGames, &mut page))
            .unwrap();

        let buffer = terminal.backend().buffer();
        for (row, outcome) in [
            (1, Some(GameOutcome::Win)),
            (2, Some(GameOutcome::Loss)),
            (3, Some(GameOutcome::Draw)),
            (4, None),
        ] {
            let label = outcome_label(outcome);
            let text: String = (0..32)
                .map(|column| buffer[(column, row)].symbol())
                .collect();
            let column = text.find(label.content.as_ref()).unwrap() as u16;
            let cell = &buffer[(column, row)];
            assert_eq!(cell.fg, label.style.fg.unwrap_or(Color::Reset));
            assert_eq!(cell.bg, if row == 1 { SELECTION } else { Color::Reset });
        }
        assert_eq!(buffer[(0, 1)].symbol(), "›");
        assert_eq!(buffer[(2, 1)].bg, SELECTION);
    }
}

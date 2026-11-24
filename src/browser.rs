use std::{
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
    widgets::{Block, Borders, Paragraph, Row, Table, TableState, Wrap},
};

use crate::api::{Entry, Records, ResponseMessage, View};

const ACCENT: Color = Color::Rgb(24, 92, 63);
const WARNING: Color = Color::Rgb(122, 76, 8);
const DETAIL_PANE_WIDTH: u16 = 90;

#[derive(Default)]
struct Page {
    records: Option<Records>,
    loading: bool,
    error: Option<String>,
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

struct Browser {
    view: View,
    pages: [Page; 3],
    requests: Sender<View>,
    notice: Option<&'static str>,
    narrow_details: bool,
    detail_offset: usize,
}

impl Browser {
    fn page(&self) -> &Page {
        &self.pages[self.view.index()]
    }

    fn page_mut(&mut self) -> &mut Page {
        &mut self.pages[self.view.index()]
    }

    fn refresh(&mut self) {
        if self.page().loading {
            return;
        }
        let sent = self.requests.send(self.view).is_ok();
        let page = self.page_mut();
        page.loading = sent;
        page.error = if sent {
            None
        } else {
            Some("The API worker stopped. Restart ChessStudy.".into())
        };
    }

    fn switch(&mut self, view: View) {
        self.view = view;
        self.narrow_details = false;
        self.detail_offset = 0;
        if self.page().records.is_none() && self.page().error.is_none() {
            self.refresh();
        }
    }

    fn receive(&mut self, (view, response): ResponseMessage) {
        let page = &mut self.pages[view.index()];
        page.loading = false;
        match response {
            Ok(records) => page.replace(records),
            Err(error) => page.error = Some(error.to_string()),
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
                self.detail_offset = self
                    .detail_offset
                    .saturating_add_signed(amount)
                    .min(entry.details.len() + 1);
                return true;
            }
        }
        match key.code {
            KeyCode::Char('q') => return false,
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
            KeyCode::PageDown => self.page_mut().step(page_step),
            KeyCode::Home => self.page_mut().step(isize::MIN),
            KeyCode::End => self.page_mut().step(isize::MAX),
            KeyCode::Char('r') => self.refresh(),
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
        if area.width < 32 || area.height < 12 {
            frame.render_widget(Paragraph::new("Resize to 32 × 12 or press q."), area);
            return;
        }
        let page = self.page();
        let stale_error = page.error.as_deref().filter(|_| page.selected().is_some());
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
            Paragraph::new("ChessStudy").style(Style::new().bold()),
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
                        .fg(Color::White)
                        .bg(ACCENT)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::new()
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
                    (_, 1) => "completed game",
                    (_, _) => "completed games",
                };
                format!("{}  ·  {count} {item}", records.username)
            },
        );
        let mut status_lines = vec![Line::raw(summary)];
        if let Some(error) = stale_error {
            let warning = Style::new().fg(Color::White).bg(WARNING);
            status_lines.push(Line::styled("Showing previous results.", warning));
            status_lines.push(Line::styled(error, warning));
        } else if page.loading {
            status_lines.push(Line::raw(if page.records.is_some() {
                "Refreshing…"
            } else {
                "Loading…"
            }));
        }
        frame.render_widget(
            Paragraph::new(status_lines).wrap(Wrap { trim: true }),
            status,
        );

        if let Some(selected) = page.selected() {
            if split {
                let [list, details] =
                    Layout::horizontal([Constraint::Percentage(59), Constraint::Fill(1)])
                        .areas(content);
                render_details(frame, details, selected, true, 0);
                render_entries(frame, list, self.view, self.page_mut());
            } else if self.narrow_details {
                render_details(frame, content, selected, false, self.detail_offset);
            } else {
                render_entries(frame, content, self.view, self.page_mut());
            }
        } else {
            let (text, style) = if let Some(error) = &page.error {
                (error.as_str(), Style::new().fg(Color::White).bg(WARNING))
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

        let movement = if !split && self.narrow_details {
            "Scroll"
        } else {
            "Move"
        };
        let short = format!("↑↓ {movement}  ↵ Open  r Reload q Quit");
        let detail = if self.narrow_details {
            "d Back"
        } else {
            "d Details"
        };
        let controls = if split {
            vec![
                Line::raw(format!("{short}  Tab Next view")),
                Line::raw(self.notice.unwrap_or("")),
            ]
        } else {
            let short = format!("↑↓ {movement} ↵ Open r Reload q Quit");
            let navigation = if self.page().selected().is_some() {
                format!("{detail}  Tab Next view")
            } else {
                "Tab Next view".into()
            };
            vec![
                Line::raw(short),
                Line::raw(self.notice.map_or(navigation, str::to_owned)),
            ]
        };
        frame.render_widget(Paragraph::new(controls), footer);
    }
}

fn render_entries(frame: &mut Frame, area: Rect, view: View, page: &mut Page) {
    let studies = view == View::LichessStudies;
    let show_speed = !studies && area.width >= 48;
    let mut columns = vec![Constraint::Fill(1)];
    let mut headings = vec![if studies { "Study" } else { "White vs Black" }];
    if show_speed {
        columns.push(Constraint::Length(10));
        headings.push("Speed");
    }
    columns.push(Constraint::Length(10));
    headings.push(if studies { "Updated" } else { "Date" });
    let rows = page.records.as_ref().unwrap().entries.iter().map(|entry| {
        let mut cells = vec![entry.title.clone()];
        if show_speed {
            cells.push(entry.category.clone());
        }
        cells.push(entry.date.map_or_else(
            || "Unknown".into(),
            |date| date.format("%Y-%m-%d").to_string(),
        ));
        Row::new(cells)
    });
    let table = Table::new(rows, columns)
        .header(Row::new(headings).style(Style::new().bold()))
        .column_spacing(2)
        .highlight_symbol("› ")
        .row_highlight_style(Style::new().fg(Color::White).bg(ACCENT).bold());
    frame.render_stateful_widget(table, area, &mut page.selection);
}

fn render_details(frame: &mut Frame, area: Rect, entry: &Entry, bordered: bool, skip: usize) {
    let mut lines = vec![
        Line::styled(entry.title.as_str(), Style::new().bold()),
        Line::raw(""),
    ];
    lines.extend(entry.details.iter().map(|(label, value)| {
        Line::from(vec![
            Span::styled(format!("{label: <8}"), Style::new().bold()),
            Span::raw(value.as_str()),
        ])
    }));
    let details =
        Paragraph::new(lines.into_iter().skip(skip).collect::<Vec<_>>()).wrap(Wrap { trim: true });
    let details = if bordered {
        details.block(
            Block::new()
                .borders(Borders::LEFT)
                .padding(ratatui::widgets::Padding::left(2)),
        )
    } else {
        details
    };
    frame.render_widget(details, area);
}

pub fn run(
    terminal: &mut DefaultTerminal,
    requests: Sender<View>,
    responses: Receiver<ResponseMessage>,
) -> io::Result<()> {
    let mut browser = Browser {
        view: View::ChessComGames,
        pages: std::array::from_fn(|_| Page::default()),
        requests,
        notice: None,
        narrow_details: false,
        detail_offset: 0,
    };
    browser.refresh();
    loop {
        while let Ok(message) = responses.try_recv() {
            browser.receive(message);
        }
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
                details: Vec::new(),
            })
            .collect();
        Records {
            username: "test-player".into(),
            entries,
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
        };
        browser.receive((View::LichessStudies, Ok(records(&["First"]))));
        browser.refresh();
        browser.refresh();
        assert!(matches!(pending.try_recv(), Ok(View::LichessStudies)));
        assert!(pending.try_recv().is_err());

        browser.receive((
            View::LichessStudies,
            Err(anyhow::anyhow!("Could not connect")),
        ));
        assert!(browser.page().selected().is_some());
        assert_eq!(browser.page().error.as_deref(), Some("Could not connect"));
        assert!(!browser.page().loading);
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
        };
        browser.receive((View::LichessStudies, Ok(records(&["First"]))));
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
}

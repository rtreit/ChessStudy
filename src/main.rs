mod api;
mod browser;
mod config;

use anyhow::Result;
use config::Config;

fn main() -> Result<()> {
    let config = Config::load()?;
    let (chapter_requests, chapter_results) = api::spawn_chapters(config.clone());
    let (imports, import_results) = api::spawn_importer(config.clone());
    let api = api::Api::new(config)?;
    let (requests, responses) = api.spawn();
    Ok(ratatui::run(|terminal| {
        browser::run(
            terminal,
            requests,
            responses,
            imports,
            import_results,
            chapter_requests,
            chapter_results,
        )
    })?)
}

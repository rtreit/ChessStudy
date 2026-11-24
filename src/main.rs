mod api;
mod browser;
mod config;

use anyhow::Result;
use config::Config;

fn main() -> Result<()> {
    let api = api::Api::new(Config::load()?)?;
    let (requests, responses) = api.spawn();
    Ok(ratatui::run(|terminal| {
        browser::run(terminal, requests, responses)
    })?)
}

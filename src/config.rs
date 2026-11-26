use std::{env, io::ErrorKind};

use anyhow::{Context, Result, ensure};

#[derive(Clone)]
pub struct Config {
    pub chesscom_username: Option<String>,
    pub lichess_username: Option<String>,
    pub lichess_token: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        match dotenvy::from_filename(".env") {
            Ok(_) => {}
            Err(dotenvy::Error::Io(error)) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => {
                anyhow::bail!("Could not read .env. Check its permissions and KEY=value syntax.")
            }
        }

        Ok(Self {
            chesscom_username: username("CHESSCOM_USERNAME")?,
            lichess_username: username("LICHESS_USERNAME")?,
            lichess_token: setting("LICHESS_TOKEN")?,
        })
    }
}

fn setting(key: &str) -> Result<Option<String>> {
    match env::var(key) {
        Ok(value) => Ok(Some(value.trim().to_owned()).filter(|value| !value.is_empty())),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(anyhow::anyhow!("{key} must contain valid Unicode.")),
    }
}

fn username(key: &str) -> Result<Option<String>> {
    let value = setting(key).with_context(|| format!("Could not read {key}"))?;
    if let Some(name) = &value {
        ensure!(
            name.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
            "{key} must contain only letters, numbers, hyphens, or underscores."
        );
    }
    Ok(value)
}

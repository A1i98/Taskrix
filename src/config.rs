use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::i18n;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskDestination {
    Inbox,
    Journal,
}

pub use crate::i18n::Lang;

#[derive(Debug, Clone)]
pub struct Config {
    pub homeserver: String,
    pub user: String,
    pub admin: String,
    pub password: Option<String>,
    pub vault: PathBuf,
    pub inbox: String,
    pub journal_dir: String,
    pub tasks_heading: Option<String>,
    pub ignore_dirs: Vec<String>,
    pub include_dirs: Vec<String>,
    pub task_destination: TaskDestination,
    pub lang: Lang,
    pub store: String,
    pub remind_room: Option<String>,
    pub digest_at: (u32, u32),
    pub allowed: Vec<String>,
}

fn parse_digest_at(value: &str, lang: Lang) -> Result<(u32, u32)> {
    let (h, m) = value
        .split_once(':')
        .context(i18n::digest_format_invalid(lang))?;
    let hour: u32 = h.trim().parse().context(i18n::digest_hour_invalid(lang))?;
    let minute: u32 = m
        .trim()
        .parse()
        .context(i18n::digest_minute_invalid(lang))?;
    if hour > 23 || minute > 59 {
        anyhow::bail!(i18n::digest_out_of_range(lang));
    }
    Ok((hour, minute))
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let lang_raw = std::env::var("LANG").unwrap_or_else(|_| "en".to_string());
        let lang = Lang::from_env_value(&lang_raw);

        let homeserver =
            std::env::var("MATRIX_HS").with_context(|| i18n::env_missing(lang, "MATRIX_HS"))?;
        let user =
            std::env::var("MATRIX_USER").with_context(|| i18n::env_missing(lang, "MATRIX_USER"))?;

        let password = std::env::var("MATRIX_PASS").ok().filter(|s| !s.is_empty());
        if password.is_none() {
            anyhow::bail!(i18n::matrix_pass_required(lang));
        }

        let vault = PathBuf::from(
            std::env::var("VAULT").with_context(|| i18n::env_missing(lang, "VAULT"))?,
        );
        let inbox = std::env::var("INBOX_FILE").unwrap_or_else(|_| "Inbox.md".to_string());
        let journal_dir = std::env::var("JOURNAL_DIR").unwrap_or_else(|_| "journals".to_string());
        let tasks_heading = std::env::var("TASKS_HEADING")
            .ok()
            .filter(|s| !s.is_empty());
        let ignore_dirs: Vec<String> = std::env::var("IGNORE_DIRS")
            .unwrap_or_else(|_| ".obsidian,.trash,.git,templates".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let include_dirs: Vec<String> = std::env::var("INCLUDE_DIRS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let task_destination = match std::env::var("TASK_DEST")
            .unwrap_or_else(|_| "inbox".to_string())
            .to_lowercase()
            .as_str()
        {
            "journal" | "daily" => TaskDestination::Journal,
            _ => TaskDestination::Inbox,
        };
        let store = std::env::var("STORE").unwrap_or_else(|_| "./taskrix-store".to_string());

        let remind_room = std::env::var("REMIND_ROOM").ok().filter(|s| !s.is_empty());

        let digest_at_raw = std::env::var("DIGEST_AT").unwrap_or_else(|_| "08:00".to_string());
        let digest_at = parse_digest_at(&digest_at_raw, lang)?;

        let allowed = std::env::var("ALLOWED").unwrap_or_else(|_| user.clone());
        let allowed: Vec<String> = allowed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let admin = std::env::var("ADMIN_USER").unwrap_or_else(|_| user.clone());

        Ok(Self {
            homeserver,
            user,
            admin,
            password,
            vault,
            inbox,
            journal_dir,
            tasks_heading,
            ignore_dirs,
            include_dirs,
            task_destination,
            lang,
            store,
            remind_room,
            digest_at,
            allowed,
        })
    }

    pub fn is_allowed(&self, sender: &str) -> bool {
        self.allowed.iter().any(|u| u == sender)
    }
}

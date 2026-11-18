mod config;
mod i18n;
mod parse;
mod tasks;

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Local, NaiveTime, TimeZone};
use config::Config;
use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::{
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        },
        RoomId,
    },
    Client, LoopCtrl,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn send_plain(room: &Room, body: &str) {
    let content = RoomMessageEventContent::text_plain(body);
    if let Err(e) = room.send(content).await {
        warn!("Failed to send message: {e}");
    }
}

async fn handle_message(cfg: Arc<Config>, room: Room, event: OriginalSyncRoomMessageEvent, start_ts: u64, last_lists: Arc<tokio::sync::Mutex<HashMap<String, Vec<tasks::Task>>>>) {
    let room_id = room.room_id().to_string();
    // Ignore messages from self
    if event.sender.as_str() == cfg.user {
        return;
    }
    // Allowlist check
    if !cfg.is_allowed(event.sender.as_str()) {
        return;
    }
    // Ignore old messages (replayed on sync)
    let msg_ts: u64 = event.origin_server_ts.0.try_into().unwrap_or(0);
    if msg_ts < start_ts.saturating_sub(120_000) {
        return;
    }

    let MessageType::Text(text_content) = &event.content.msgtype else {
        return;
    };
    let body = text_content.body.trim();
    if body.is_empty() {
        return;
    }

    let lower = body.to_lowercase();

    if lower == "!help" || lower == "!راهنما" {
        send_plain(&room, i18n::help(cfg.lang)).await;
        return;
    }
    if lower == "!ping" {
        send_plain(&room, i18n::pong()).await;
        return;
    }
    if lower == "!version" || lower == "!about" || lower == "!نسخه" {
        send_plain(&room, &format!("Taskrix v{}", env!("CARGO_PKG_VERSION"))).await;
        return;
    }

    if lower == "!roomid" || lower == "!اتاق" {
        send_plain(&room, room.room_id().as_str()).await;
        return;
    }

    if lower == "!today" || lower == "!امروز" {
        let digest = tasks::digest(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs);
        send_plain(&room, &digest).await;
        return;
    }

    if lower == "!list" || lower == "!tasks" || lower == "!لیست" {
        let list = tasks::list_default(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs);
        if list.is_empty() {
            send_plain(&room, i18n::no_open_tasks(cfg.lang)).await;
        } else {
            let header = if cfg.lang == i18n::Lang::Fa {
                format!("تسک‌های مهم (overdue + امروز + تا ۴۵ روز آینده)، {} مورد:\n", list.len())
            } else {
                format!("Key tasks (overdue + today + next 45 days), {} items:\n", list.len())
            };
            let mut out = header;
            for (i, t) in list.iter().take(20).enumerate() {
                let due = t.due.map(|d| format!(" ({})", d.format("%Y-%m-%d"))).unwrap_or_default();
                let sched = t.scheduled.map(|d| format!(" ⏳{}", d.format("%Y-%m-%d"))).unwrap_or_default();
                out.push_str(&format!("{}. {}{}{}\n", i + 1, t.text, due, sched));
            }
            if list.len() > 20 {
                out.push_str(&i18n::and_more(cfg.lang, list.len() - 20));
            }
            last_lists.lock().await.insert(room_id.clone(), list);
            send_plain(&room, &out).await;
        }
        return;
    }
    if lower == "!list all" || lower == "!tasks all" {
        let list = tasks::list_all(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs);
        if list.is_empty() {
            send_plain(&room, i18n::no_open_tasks(cfg.lang)).await;
        } else {
            let mut out = if cfg.lang == i18n::Lang::Fa {
                format!("همهٔ تسک‌های باز ({} مورد)، نمایش تا ۲۰:\n", list.len())
            } else {
                format!("All open tasks ({}), showing up to 20:\n", list.len())
            };
            for (i, t) in list.iter().take(20).enumerate() {
                let due = t.due.map(|d| format!(" ({})", d.format("%Y-%m-%d"))).unwrap_or_default();
                let sched = t.scheduled.map(|d| format!(" ⏳{}", d.format("%Y-%m-%d"))).unwrap_or_default();
                out.push_str(&format!("{}. {}{}{}\n", i + 1, t.text, due, sched));
            }
            if list.len() > 20 {
                out.push_str(&i18n::and_more(cfg.lang, list.len() - 20));
            }
            last_lists.lock().await.insert(room_id.clone(), list);
            send_plain(&room, &out).await;
        }
        return;
    }
    if lower == "!list overdue" {
        let list = tasks::list_overdue(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs);
        if list.is_empty() {
            let msg = if cfg.lang == i18n::Lang::Fa { "تسک عقب‌افتاده‌ای نیست." } else { "No overdue tasks." };
            send_plain(&room, msg).await;
        } else {
            let mut out = if cfg.lang == i18n::Lang::Fa { format!("عقب‌افتاده ({}):\n", list.len()) } else { format!("Overdue ({}):\n", list.len()) };
            for (i, t) in list.iter().take(20).enumerate() {
                let due = t.due.map(|d| format!(" ({})", d.format("%Y-%m-%d"))).unwrap_or_default();
                out.push_str(&format!("{}. {}{}\n", i + 1, t.text, due));
            }
            last_lists.lock().await.insert(room_id.clone(), list);
            send_plain(&room, &out).await;
        }
        return;
    }
    if lower == "!list today" {
        let list = tasks::list_today(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs);
        if list.is_empty() {
            let msg = if cfg.lang == i18n::Lang::Fa { "برای امروز تسکی نیست." } else { "No tasks due today." };
            send_plain(&room, msg).await;
        } else {
            let mut out = if cfg.lang == i18n::Lang::Fa { format!("امروز ({}):\n", list.len()) } else { format!("Today ({}):\n", list.len()) };
            for (i, t) in list.iter().take(20).enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, t.text));
            }
            last_lists.lock().await.insert(room_id.clone(), list);
            send_plain(&room, &out).await;
        }
        return;
    }

    // admin-only config show (minimal)
    if lower == "!config" || lower == "!config show" {
        if event.sender.as_str() != cfg.admin {
            send_plain(&room, i18n::not_authorized(cfg.lang)).await;
        } else {
            let dest = match cfg.task_destination {
                config::TaskDestination::Inbox => "inbox",
                config::TaskDestination::Journal => "journal",
            };
            let msg = i18n::config_show(
                cfg.lang,
                dest,
                &cfg.journal_dir,
                &cfg.inbox,
                cfg.digest_at.0,
                cfg.digest_at.1,
                &cfg.admin,
            );
            send_plain(&room, &msg).await;
        }
        return;
    }

    if let Some(q) = lower.strip_prefix("!done ") {
        let query = q.trim();
        if query.is_empty() {
            send_plain(&room, i18n::done_usage(cfg.lang)).await;
            return;
        }
        // Try to parse as number (index from last list)
        if let Ok(n) = query.parse::<usize>() {
            if n == 0 {
                send_plain(&room, if cfg.lang == i18n::Lang::Fa { "شماره باید از ۱ شروع شود." } else { "Index must start from 1." }).await;
                return;
            }
            let lists = last_lists.lock().await;
            if let Some(list) = lists.get(&room_id) {
                if n <= list.len() {
                    let task = &list[n - 1];
                    match std::fs::read_to_string(&task.file) {
                        Ok(content) => {
                            let done_line = task.line.replacen("- [ ]", "- [x]", 1);
                            if done_line == task.line {
                                send_plain(&room, i18n::done_not_found(cfg.lang)).await;
                            } else {
                                let new_content = content.replacen(&task.line, &done_line, 1);
                                if new_content != content {
                                    let _ = std::fs::write(&task.file, &new_content);
                                    send_plain(&room, &i18n::done_ok(cfg.lang, &task.text)).await;
                                } else {
                                    send_plain(&room, i18n::done_not_found(cfg.lang)).await;
                                }
                            }
                        }
                        Err(_) => send_plain(&room, i18n::done_not_found(cfg.lang)).await,
                    }
                } else {
                    let msg = if cfg.lang == i18n::Lang::Fa { format!("فقط {} مورد در لیست آخر بود.", list.len()) } else { format!("Last list had only {} items.", list.len()) };
                    send_plain(&room, &msg).await;
                }
            } else {
                let msg = if cfg.lang == i18n::Lang::Fa { "اول یک لیست بگیر (مثل !list) بعد شماره بده." } else { "Get a list first (e.g. !list), then use its number." };
                send_plain(&room, msg).await;
            }
            return;
        }
        match tasks::complete_task(&cfg.vault, &cfg.ignore_dirs, &cfg.include_dirs, query) {
            Some(t) => send_plain(&room, &i18n::done_ok(cfg.lang, &t)).await,
            None => send_plain(&room, i18n::done_not_found(cfg.lang)).await,
        }
        return;
    }

    // If message starts with !task, strip it; otherwise treat whole message as task
    let raw_task = if lower.starts_with("!task") {
        body[5..].trim()
    } else {
        body
    };

    if raw_task.is_empty() {
        let msg = if cfg.lang == i18n::Lang::Fa {
            "متن تسک خالی است."
        } else {
            "Task text is empty."
        };
        send_plain(&room, msg).await;
        return;
    }

    let parsed = parse::parse(raw_task);
    if parsed.text.trim().is_empty() {
        let msg = if cfg.lang == i18n::Lang::Fa {
            "نتوانستم عنوان تسک را از پیام استخراج کنم."
        } else {
            "Could not parse a task title from your message."
        };
        send_plain(&room, msg).await;
        return;
    }

    let line = parse::build_line(&parsed);
    let result = if cfg.task_destination == config::TaskDestination::Journal {
        let date = parsed
            .due
            .unwrap_or_else(|| chrono::Local::now().date_naive());
        tasks::append_task_to_journal(
            &cfg.vault,
            &cfg.journal_dir,
            date,
            &line,
            cfg.tasks_heading.as_deref(),
        )
        .map(|p| i18n::task_added_journal(cfg.lang, &p.display().to_string(), &line))
    } else {
        tasks::append_task(&cfg.vault, &cfg.inbox, &line)
            .map(|_| i18n::task_added_inbox(cfg.lang, &line))
    };
    match result {
        Ok(msg) => send_plain(&room, &msg).await,
        Err(e) => send_plain(&room, &i18n::task_write_failed(cfg.lang, &e.to_string())).await,
    }
}

fn seconds_until_next(h: u32, m: u32) -> u64 {
    let now = Local::now();
    let time = match NaiveTime::from_hms_opt(h, m, 0) {
        Some(t) => t,
        None => return 60, // fallback
    };
    let today_dt = Local.from_local_datetime(&now.date_naive().and_time(time));
    let mut next = match today_dt.single() {
        Some(dt) => dt,
        None => now + ChronoDuration::hours(1),
    };
    if next <= now {
        let tomorrow = now.date_naive() + ChronoDuration::days(1);
        let tomorrow_dt = Local.from_local_datetime(&tomorrow.and_time(time));
        next = match tomorrow_dt.single() {
            Some(dt) => dt,
            None => now + ChronoDuration::hours(24),
        };
    }
    let diff = (next - now).num_seconds();
    if diff < 1 {
        1
    } else {
        diff as u64
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (ignore errors if not found)
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("taskrix=info".parse()?),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Arc::new(Config::from_env()?);
    info!("Starting Taskrix, homeserver={}", cfg.homeserver);
    type LastLists = Arc<tokio::sync::Mutex<HashMap<String, Vec<tasks::Task>>>>;
    let last_lists: LastLists = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let client = Client::builder()
        .homeserver_url(&cfg.homeserver)
        .sqlite_store(&cfg.store, None)
        .build()
        .await?;

    // Login
    if let Some(password) = &cfg.password {
        client
            .matrix_auth()
            .login_username(&cfg.user, password)
            .initial_device_display_name("Taskrix")
            .send()
            .await?;
        info!("Logged in as {}", cfg.user);
    } else {
        anyhow::bail!(
            "MATRIX_PASS must be set (token restore is environment-specific for this build)"
        );
    }

    let start_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Message handler
    client.add_event_handler({
        let cfg = cfg.clone();
        let last_lists = last_lists.clone();
        move |ev: OriginalSyncRoomMessageEvent, room: Room| {
            let cfg = cfg.clone();
            let last_lists = last_lists.clone();
            async move {
                handle_message(cfg, room, ev, start_ts, last_lists).await;
            }
        }
    });

    // Auto-join invited rooms
    client.add_event_handler(
        |ev: StrippedRoomMemberEvent, room: Room, client: Client| async move {
            if let Some(user_id) = client.user_id() {
                if ev.state_key == *user_id {
                    let room_id = room.room_id();
                    if let Err(e) = client.join_room_by_id(room_id).await {
                        warn!("Failed to join room {}: {e}", room_id);
                    } else {
                        info!("Joined room {}", room_id);
                    }
                }
            }
        },
    );

    // Daily digest task
    if let Some(room_id_str) = cfg.remind_room.clone() {
        let client_clone = client.clone();
        let cfg_clone = cfg.clone();
        tokio::spawn(async move {
            loop {
                let wait_secs = seconds_until_next(cfg_clone.digest_at.0, cfg_clone.digest_at.1);
                info!("Next digest in {} seconds", wait_secs);
                tokio::time::sleep(Duration::from_secs(wait_secs)).await;

                match RoomId::parse(&room_id_str) {
                    Ok(rid) => {
                        if let Some(room) = client_clone.get_room(&rid) {
                            let digest = tasks::digest(&cfg_clone.vault, &cfg_clone.ignore_dirs, &cfg_clone.include_dirs);
                            let content = RoomMessageEventContent::text_plain(digest);
                            if let Err(e) = room.send(content).await {
                                warn!("Failed to send digest: {e}");
                            } else {
                                info!("Digest sent to {}", room_id_str);
                            }
                        } else {
                            warn!("Digest room not found in joined rooms: {}", room_id_str);
                        }
                    }
                    Err(e) => {
                        warn!("Invalid REMIND_ROOM id '{}': {e}", room_id_str);
                    }
                }
            }
        });
    }

    info!("Starting sync loop...");
    let sync_settings = SyncSettings::default();
    client
        .sync_with_callback(sync_settings, |_| async { LoopCtrl::Continue })
        .await?;

    Ok(())
}

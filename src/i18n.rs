#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fa,
}

impl Lang {
    pub fn from_env_value(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "fa" | "fa-ir" | "persian" | "فارسی" => Lang::Fa,
            _ => Lang::En,
        }
    }
}

pub fn help(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => {
            "**تسکریکس**\n\
- هر متنی بفرستی → ساخت تسک\n\
- `@+3` یا `@2026-11-20` یا `due::2026-11-20` → سررسید\n\
- `#daily|weekly|monthly` یا `repeat::...` → تکرار\n\
- `! !! !!!` یا `priority::high` → اولویت\n\
- `!today` / `!امروز` → خلاصه امروز\n\
- `!list` (مهم‌ترین‌ها) | `!list all` | `!list today` | `!list overdue`\n\
- `!done متن` → انجام شدن تسک\n\
- `!ping` / `!version` / `!roomid` / `!help`"
        }
        Lang::En => {
            "**Taskrix**\n\
- Send any text -> creates a task\n\
- `@+3` / `@2026-11-20` / `due::2026-11-20` -> due date\n\
- `#daily|weekly|monthly` or `repeat::...` -> recurrence\n\
- `! !! !!!` or `priority::high` -> priority\n\
- `!today` -> digest\n\
- `!list` (key) | `!list all` | `!list today` | `!list overdue`\n\
- `!done <text>` -> complete a task\n\
- `!ping` / `!version` / `!roomid` / `!help`"
        }
    }
}

pub fn pong() -> &'static str {
    "🏓 pong"
}

pub fn no_open_tasks(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "تسک بازی وجود ندارد.",
        Lang::En => "No open tasks.",
    }
}

pub fn and_more(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Fa => format!("...و {} تای دیگر.", n),
        Lang::En => format!("...and {} more.", n),
    }
}

pub fn not_authorized(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "دسترسی ندارید (فقط ادمین).",
        Lang::En => "Not authorized (admin only).",
    }
}

pub fn config_show(
    lang: Lang,
    task_dest: &str,
    journal_dir: &str,
    inbox_file: &str,
    dh: u32,
    dm: u32,
    admin: &str,
) -> String {
    match lang {
        Lang::Fa => format!(
            "کانفیگ (env): TASK_DEST={}، JOURNAL_DIR={}، INBOX_FILE={}، DIGEST_AT={:02}:{:02}، ADMIN={}",
            task_dest, journal_dir, inbox_file, dh, dm, admin
        ),
        Lang::En => format!(
            "Config (env): TASK_DEST={}, JOURNAL_DIR={}, INBOX_FILE={}, DIGEST_AT={:02}:{:02}, ADMIN={}",
            task_dest, journal_dir, inbox_file, dh, dm, admin
        ),
    }
}

pub fn done_usage(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "استفاده: !done بخش از متن تسک",
        Lang::En => "Usage: !done <part of task text>",
    }
}

pub fn done_ok(lang: Lang, text: &str) -> String {
    match lang {
        Lang::Fa => format!("✅ انجام شد: {}", text),
        Lang::En => format!("✅ Completed: {}", text),
    }
}

pub fn done_not_found(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "تسک matching پیدا نشد.",
        Lang::En => "No matching open task found.",
    }
}

pub fn task_added_inbox(lang: Lang, line: &str) -> String {
    match lang {
        Lang::Fa => format!("➕ اضافه شد: {}", line),
        Lang::En => format!("➕ Added: {}", line),
    }
}

pub fn task_added_journal(lang: Lang, path: &str, line: &str) -> String {
    match lang {
        Lang::Fa => format!("➕ اضافه شد به {}: {}", path, line),
        Lang::En => format!("➕ Added to {}: {}", path, line),
    }
}

pub fn task_write_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Fa => format!("❌ خطا در ذخیره: {}", err),
        Lang::En => format!("❌ Failed to write task: {}", err),
    }
}

// Config/startup messages
pub fn env_missing(lang: Lang, key: &str) -> String {
    match lang {
        Lang::Fa => format!("متغیر محیطی {} تنظیم نشده است.", key),
        Lang::En => format!("Environment variable {} is not set.", key),
    }
}

pub fn digest_format_invalid(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "DIGEST_AT باید به صورت HH:MM باشد (مثلاً 08:00).",
        Lang::En => "DIGEST_AT must be in HH:MM format (e.g. 08:00).",
    }
}

pub fn digest_hour_invalid(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "ساعت در DIGEST_AT معتبر نیست.",
        Lang::En => "Hour in DIGEST_AT is invalid.",
    }
}

pub fn digest_minute_invalid(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "دقیقه در DIGEST_AT معتبر نیست.",
        Lang::En => "Minute in DIGEST_AT is invalid.",
    }
}

pub fn digest_out_of_range(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "DIGEST_AT خارج از بازهٔ معتبر است (۰۰:۰۰ تا ۲۳:۵۹).",
        Lang::En => "DIGEST_AT is out of range (00:00 to 23:59).",
    }
}

pub fn matrix_pass_required(lang: Lang) -> &'static str {
    match lang {
        Lang::Fa => "MATRIX_PASS باید تنظیم شود.",
        Lang::En => "MATRIX_PASS must be set.",
    }
}

use chrono::{Local, NaiveDate};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Task {
    pub file: PathBuf,
    pub line: String,
    pub text: String,
    pub due: Option<NaiveDate>,
    pub scheduled: Option<NaiveDate>,
}

fn task_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- [ ]")
        .or_else(|| trimmed.strip_prefix("* [ ]"))?;
    Some(rest.trim())
}

fn due_of(line: &str) -> Option<NaiveDate> {
    if let Some(idx) = line.find('📅') {
        let tail = line[idx + '📅'.len_utf8()..].trim_start();
        let date_str: String = tail.chars().take_while(|c| !c.is_whitespace()).collect();
        if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            return Some(d);
        }
    }
    if let Some(idx) = line.find("due::") {
        let tail = line[idx + "due::".len()..].trim_start();
        let date_str: String = tail
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ']')
            .collect();
        if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
}

fn scheduled_of(line: &str) -> Option<NaiveDate> {
    if let Some(idx) = line.find('⏳') {
        let tail = line[idx + '⏳'.len_utf8()..].trim_start();
        let date_str: String = tail
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ']')
            .collect();
        if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            return Some(d);
        }
    }
    if let Some(idx) = line.find("scheduled::") {
        let tail = line[idx + "scheduled::".len()..].trim_start();
        let date_str: String = tail
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ']')
            .collect();
        if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
}

pub fn append_task(vault: &Path, inbox: &str, line: &str) -> io::Result<()> {
    let path = vault.join(inbox);
    append_line_to_file(&path, line, Some("# Inbox\n\n"))
}

fn append_line_to_file(path: &Path, line: &str, header_if_new: Option<&str>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let is_new = !path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if is_new {
        if let Some(h) = header_if_new {
            file.write_all(h.as_bytes())?;
        }
    }
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn append_task_to_journal(
    vault: &Path,
    journal_dir: &str,
    date: chrono::NaiveDate,
    line: &str,
    heading: Option<&str>,
) -> io::Result<std::path::PathBuf> {
    let filename = format!("{}.md", date.format("%Y-%m-%d"));
    let path = vault.join(journal_dir).join(filename);
    let header = heading.map(|h| format!("# {}\n\n{}\n\n", date.format("%Y-%m-%d"), h));
    append_line_to_file(&path, line, header.as_deref())?;
    Ok(path)
}

fn walk_markdown_files(dir: &Path, out: &mut Vec<PathBuf>, ignore_dirs: &[String]) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if name.starts_with('.') || ignore_dirs.iter().any(|d| d == &name) {
            continue;
        }
        if path.is_dir() {
            walk_markdown_files(&path, out, ignore_dirs);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

pub fn open_tasks(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> Vec<Task> {
    let mut files = Vec::new();
    if !include_dirs.is_empty() {
        for inc in include_dirs {
            let p = vault.join(inc);
            if p.is_dir() {
                walk_markdown_files(&p, &mut files, ignore_dirs);
            } else if p.is_file() && p.extension().is_some_and(|e| e == "md") {
                files.push(p);
            }
        }
    } else {
        walk_markdown_files(vault, &mut files, ignore_dirs);
    }

    let mut tasks = Vec::new();
    for file in files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        for line in content.lines() {
            if let Some(text) = task_text(line) {
                tasks.push(Task {
                    file: file.clone(),
                    line: line.to_string(),
                    text: text.to_string(),
                    due: due_of(line),
                    scheduled: scheduled_of(line),
                });
            }
        }
    }
    tasks
}

pub fn complete_task(vault: &Path, ignore_dirs: &[String], include_dirs: &[String], query: &str) -> Option<String> {
    let q = query.to_lowercase();
    for task in open_tasks(vault, ignore_dirs, include_dirs) {
        if task.line.to_lowercase().contains(&q) || task.text.to_lowercase().contains(&q) {
            let content = fs::read_to_string(&task.file).ok()?;
            let done_line = task.line.replacen("- [ ]", "- [x]", 1);
            if done_line == task.line {
                continue;
            }
            let new_content = content.replacen(&task.line, &done_line, 1);
            fs::write(&task.file, new_content).ok()?;
            return Some(task.text);
        }
    }
    None
}

pub fn digest(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> String {
    let today = Local::now().date_naive();
    let tasks = open_tasks(vault, ignore_dirs, include_dirs);

    let mut overdue: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.due.is_some_and(|d| d < today))
        .collect();
    overdue.sort_by_key(|t| t.due);

    let due_today: Vec<&Task> = tasks.iter().filter(|t| t.due == Some(today)).collect();

    let mut nodate: Vec<&Task> = tasks.iter().filter(|t| t.due.is_none()).collect();
    nodate.sort_by(|a, b| a.text.cmp(&b.text));

    let mut out = format!(
        "**📅 {}** — {} task(s) due today, {} overdue",
        today.format("%Y-%m-%d"),
        due_today.len(),
        overdue.len()
    );

    if !overdue.is_empty() {
        out.push_str("\n\n**Overdue**");
        for t in &overdue {
            let due_str = t
                .due
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            out.push_str(&format!("\n- {}  ({})", t.text, due_str));
        }
    }

    if !due_today.is_empty() {
        out.push_str("\n\n**Today**");
        for t in &due_today {
            out.push_str(&format!("\n- {}", t.text));
        }
    }

    if !nodate.is_empty() {
        out.push_str(&format!("\n\n**No due date** ({})", nodate.len()));
        for t in nodate.iter().take(10) {
            out.push_str(&format!("\n- {}", t.text));
        }
        if nodate.len() > 10 {
            out.push_str(&format!("\n- ... and {} more", nodate.len() - 10));
        }
    }

    if overdue.is_empty() && due_today.is_empty() && nodate.is_empty() {
        out.push_str("\n\nNo open tasks found. 🎉");
    }

    out
}

pub fn list_default(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> Vec<Task> {
    let today = Local::now().date_naive();
    let end = today + chrono::Duration::days(45);
    let mut tasks: Vec<Task> = open_tasks(vault, ignore_dirs, include_dirs)
        .into_iter()
        .filter(|t| {
            let scheduled_ok = t.scheduled.is_some_and(|s| s <= today);
            let due_ok = t.due.is_some_and(|d| d <= end);
            scheduled_ok || due_ok
        })
        .collect();
    tasks.sort_by(|a, b| {
        let ad = a.due.unwrap_or(today + chrono::Duration::days(9999));
        let bd = b.due.unwrap_or(today + chrono::Duration::days(9999));
        ad.cmp(&bd).then_with(|| a.text.cmp(&b.text))
    });
    tasks
}

pub fn list_all(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> Vec<Task> {
    let mut tasks = open_tasks(vault, ignore_dirs, include_dirs);
    tasks.sort_by(|a, b| {
        match (a.due, b.due) {
            (Some(da), Some(db)) => da.cmp(&db).then_with(|| a.text.cmp(&b.text)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.text.cmp(&b.text),
        }
    });
    tasks
}

pub fn list_overdue(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> Vec<Task> {
    let today = Local::now().date_naive();
    let mut tasks: Vec<Task> = open_tasks(vault, ignore_dirs, include_dirs)
        .into_iter()
        .filter(|t| t.due.is_some_and(|d| d < today))
        .collect();
    tasks.sort_by_key(|t| t.due);
    tasks
}

pub fn list_today(vault: &Path, ignore_dirs: &[String], include_dirs: &[String]) -> Vec<Task> {
    let today = Local::now().date_naive();
    let mut tasks: Vec<Task> = open_tasks(vault, ignore_dirs, include_dirs)
        .into_iter()
        .filter(|t| t.due == Some(today))
        .collect();
    tasks.sort_by(|a, b| a.text.cmp(&b.text));
    tasks
}

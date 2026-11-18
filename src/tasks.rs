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
}

fn task_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- [ ]")
        .or_else(|| trimmed.strip_prefix("* [ ]"))?;
    Some(rest.trim())
}

fn due_of(line: &str) -> Option<NaiveDate> {
    // Try emoji style 📅
    if let Some(idx) = line.find('📅') {
        let tail = line[idx + '📅'.len_utf8()..].trim_start();
        let date_str: String = tail.chars().take_while(|c| !c.is_whitespace()).collect();
        if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            return Some(d);
        }
    }
    // Try Tasks plugin inline: due:: 2026-11-20
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

fn walk_markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue; // skip .obsidian, .git, .trash etc.
        }
        if path.is_dir() {
            walk_markdown_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

pub fn open_tasks(vault: &Path) -> Vec<Task> {
    let mut files = Vec::new();
    walk_markdown_files(vault, &mut files);

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
                });
            }
        }
    }
    tasks
}

pub fn complete_task(vault: &Path, query: &str) -> Option<String> {
    let q = query.to_lowercase();
    for task in open_tasks(vault) {
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

pub fn digest(vault: &Path) -> String {
    let today = Local::now().date_naive();
    let tasks = open_tasks(vault);

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

use chrono::{Duration, Local, NaiveDate};

#[derive(Debug, Clone)]
pub struct Parsed {
    pub text: String,
    pub due: Option<NaiveDate>,
    pub scheduled: Option<NaiveDate>,
    pub prio: Option<&'static str>,
    pub recur: Option<String>,
}

fn parse_due_value(value: &str) -> Option<NaiveDate> {
    if let Some(n) = value.strip_prefix('+') {
        let days: i64 = n.parse().ok()?;
        return Some(Local::now().date_naive() + Duration::days(days));
    }
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

fn priority_from_token(token: &str) -> Option<&'static str> {
    match token {
        "!!!" | "highest" | "🔺" => Some("🔺"),
        "!!" | "high" | "⏫" => Some("⏫"),
        "!" | "medium" | "🔼" => Some("🔼"),
        "low" | "🔽" => Some("🔽"),
        _ => None,
    }
}

/// Parse task text supporting short syntax and some Tasks plugin inline fields.
pub fn parse(raw: &str) -> Parsed {
    let (mut due, mut scheduled, mut prio, mut recur) = (None, None, None, None);
    let mut keep: Vec<String> = Vec::new();
    let words: Vec<&str> = raw.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        // Scheduled: ⏳ followed by date, or ⏳2026-...
        if word == "⏳" {
            if i + 1 < words.len() {
                if let Some(d) = parse_due_value(words[i + 1]) {
                    scheduled = Some(d);
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        if let Some(v) = word.strip_prefix("⏳") {
            if let Some(d) = parse_due_value(v) {
                scheduled = Some(d);
                i += 1;
                continue;
            }
        }
        if let Some(v) = word.strip_prefix("scheduled::") {
            if let Some(d) = parse_due_value(v) {
                scheduled = Some(d);
                i += 1;
                continue;
            }
        }

        // Short due: @+2 or @2026-11-20
        if let Some(v) = word.strip_prefix('@') {
            if let Some(d) = parse_due_value(v) {
                due = Some(d);
                i += 1;
                continue;
            }
        }

        // Inline field style: due::2026-11-20
        if let Some(v) = word.strip_prefix("due::") {
            if let Some(d) = parse_due_value(v) {
                due = Some(d);
                i += 1;
                continue;
            }
        }

        // Repeat shortcuts
        if let Some(v) = word.strip_prefix('#') {
            let r = match v {
                "daily" => Some("every day".to_string()),
                "weekly" => Some("every week".to_string()),
                "monthly" => Some("every month".to_string()),
                _ => None,
            };
            if let Some(rr) = r {
                recur = Some(rr);
                i += 1;
                continue;
            }
        }
        if word.starts_with("repeat::") {
            let v = word.trim_start_matches("repeat::").trim();
            if !v.is_empty() {
                recur = Some(v.to_string());
            }
            i += 1;
            continue;
        }

        if word.starts_with("priority::") {
            let v = word.trim_start_matches("priority::").trim();
            if let Some(p) = priority_from_token(v) {
                prio = Some(p);
            }
            i += 1;
            continue;
        }

        if let Some(p) = priority_from_token(word) {
            prio = Some(p);
            i += 1;
            continue;
        }

        keep.push(word.to_string());
        i += 1;
    }

    Parsed {
        text: keep.join(" "),
        due,
        scheduled,
        prio,
        recur,
    }
}

pub fn build_line(p: &Parsed) -> String {
    let mut line = format!("- [ ] {}", p.text);
    if let Some(r) = &p.recur {
        line.push_str(&format!(" 🔁 {r}"));
    }
    if let Some(pr) = p.prio {
        line.push_str(&format!(" {pr}"));
    }
    if let Some(d) = p.due {
        line.push_str(&format!(" 📅 {}", d.format("%Y-%m-%d")));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_task() {
        let p = parse("!! خرید نان @+2 #daily");
        assert_eq!(p.text, "خرید نان");
        assert!(p.due.is_some());
        assert_eq!(p.prio, Some("⏫"));
        assert_eq!(p.recur.as_deref(), Some("every day"));
    }

    #[test]
    fn parses_due_inline() {
        let p = parse("task due::2026-12-01");
        assert_eq!(p.text, "task");
        assert_eq!(p.due.map(|d| d.to_string()).as_deref(), Some("2026-12-01"));
    }
}

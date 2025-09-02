use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};
use crate::backend::utility;
use crate::backend::journals;
use chrono::{NaiveDate, Duration as ChronoDuration};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CalendarItem {
    pub kind: String, // "event" | "alarm" | "reminder" | "journal"
    pub time: String,
    pub title: String,
    pub details: Option<String>,
}

/// Return events, alarms, and reminders for a date range (inclusive).
/// Scans each daily DB file between start_iso and end_iso (both inclusive) and aggregates results.
#[tauri::command]
pub fn get_calendar_range(workspace_dir: Option<String>, start_iso: String, end_iso: String) -> Result<Vec<CalendarItem>, String> {
    // Parse start and end into NaiveDate. Accept YYYY-MM-DD or RFC3339 timestamps.
    let parse_date = |s: &str| -> Option<NaiveDate> {
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") { return Some(d); }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) { return Some(dt.date_naive()); }
        None
    };

    let start_date = parse_date(&start_iso);
    let end_date = parse_date(&end_iso);

    // If parsing fails, fall back to single-db behaviour using utility::find_daily_db(None)
    let db_paths: Vec<PathBuf> = if let (Some(start), Some(end)) = (start_date, end_date) {
        let mut paths = Vec::new();
        let mut d = start;
        // base dir to search
        let base = workspace_dir.clone().unwrap_or_else(|| ".".to_string());
        while d <= end {
            // candidate filenames to check (strict)
            let prefixes = vec![format!("focusd_{}", d.format("%Y-%m-%d")), format!("focusd_{}", d.format("%Y%m%d"))];
            let exts = vec![".sqlite3", ".sqlite", ".db"];
            for pfx in prefixes.iter() {
                for ext in exts.iter() {
                    let cand = format!("{}{}", pfx, ext);
                    let cand_path = PathBuf::from(&base).join(&cand);
                    if cand_path.exists() {
                        if !paths.iter().any(|pp| pp == &cand_path) {
                            paths.push(cand_path.clone());
                        }
                    }
                }
            }
            d = d + ChronoDuration::days(1);
        }
        paths
    } else {
        match utility::find_daily_db(workspace_dir, None) {
            Some(p) => vec![p],
            None => vec![],
        }
    };

    // If no DBs found, return demo fallback
    if db_paths.is_empty() {
        return Ok(vec![
            CalendarItem { kind: "event".to_string(), time: "2025-09-10T10:00:00".to_string(), title: "Client meeting".to_string(), details: Some("Discuss roadmap".to_string()) },
            CalendarItem { kind: "alarm".to_string(), time: "2025-09-10T07:30:00".to_string(), title: "Wake up".to_string(), details: None },
            CalendarItem { kind: "reminder".to_string(), time: "".to_string(), title: "Pay bills".to_string(), details: None },
        ]);
    }

    let mut out: Vec<CalendarItem> = Vec::new();

    for db_path in db_paths {
        if let Ok(conn) = Connection::open(&db_path) {
            // EVENTS
            let col_stmt = conn.prepare("PRAGMA table_info(event)").ok();
            let mut cols = std::collections::HashSet::new();
            if let Some(mut s) = col_stmt {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }
            let time_expr = if cols.contains("event_time") { "event_time" } else if cols.contains("time") { "time" } else { "event_time" };
            let title_expr = if cols.contains("event_type") { "event_type" } else if cols.contains("type") { "type" } else { "event_type" };
            let sql = format!("SELECT {time}, {title}, COALESCE(details_json, '') FROM event WHERE DATE({time}) BETWEEN DATE(?) AND DATE(?) ORDER BY {time}", time=time_expr, title=title_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query(params![start_iso, end_iso]).map_err(|e| e.to_string())?;
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    let title: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    let details: String = r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default();
                    out.push(CalendarItem { kind: "event".to_string(), time, title, details: Some(details) });
                }
            }

            // ALARMS
            let col_stmt = conn.prepare("PRAGMA table_info(alarm)").ok();
            let mut cols = std::collections::HashSet::new();
            if let Some(mut s) = col_stmt {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }
            let time_expr = if cols.contains("alarm_time") { "alarm_time" } else if cols.contains("time") { "time" } else { "alarm_time" };
            let label_expr = if cols.contains("label") { "label" } else { "label" };
            let sql = format!("SELECT {time}, COALESCE({label}, '') FROM alarm WHERE DATE({time}) BETWEEN DATE(?) AND DATE(?) ORDER BY {time}", time=time_expr, label=label_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query(params![start_iso, end_iso]).map_err(|e| e.to_string())?;
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    let label: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    out.push(CalendarItem { kind: "alarm".to_string(), time, title: label, details: None });
                }
            }

            // REMINDERS
            if let Ok(mut stmt) = conn.prepare("SELECT COALESCE(text, '') FROM reminder WHERE DATE(COALESCE(created_at, datetime('now'))) BETWEEN DATE(?) AND DATE(?) ORDER BY COALESCE(created_at, datetime('now'))") {
                let mut rows = stmt.query(params![start_iso, end_iso]).map_err(|e| e.to_string())?;
                while let Ok(Some(r)) = rows.next() {
                    let text: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    out.push(CalendarItem { kind: "reminder".to_string(), time: "".to_string(), title: text, details: None });
                }
            }
        }
    }

    // Optionally include latest few journal entries once (from personality DB)
    if let Ok(entries) = journals::list_journal_entries(0, Some(5)) {
        for e in entries {
            out.push(CalendarItem { kind: "journal".to_string(), time: e.created_at.clone(), title: format!("Journal: {}", e.provider), details: Some(e.content) });
        }
    }

    Ok(out)
}

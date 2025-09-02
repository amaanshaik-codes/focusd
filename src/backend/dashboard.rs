use serde::{Serialize, Deserialize};
use rusqlite::{Connection};
use crate::backend::{utility};

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardAlarm {
    pub time: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardEvent {
    pub id: Option<i64>,
    pub event_type: String,
    pub event_time: String,
    pub details_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardJournalEntry {
    pub id: i64,
    pub created_at: String,
    pub provider: String,
    pub excerpt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub date: String,
    pub sessions_count: i64,
    pub focus_minutes: f64,
    pub distractions_count: i64,
    pub pending_goals: i64,
    pub pending_tasks: i64,
    pub upcoming_alarms: Vec<DashboardAlarm>,
    pub upcoming_events: Vec<DashboardEvent>,
    pub reminders: Vec<String>,
    pub recent_journal_entries: Vec<DashboardJournalEntry>,
}

fn find_db(workspace_dir: Option<String>) -> Option<std::path::PathBuf> {
    utility::find_daily_db(workspace_dir, None)
}

fn pick_alarm_time_col(conn: &Connection) -> Option<String> {
    if let Some(mut s) = conn.prepare("PRAGMA table_info(alarm)").ok() {
        if let Ok(mut rows) = s.query([]) {
            while let Ok(Some(r)) = rows.next() {
                if let Ok(name) = r.get::<_, String>(1) {
                    if name == "alarm_time" || name == "time" { return Some(name); }
                }
            }
        }
    }
    None
}

#[tauri::command]
pub fn get_dashboard_summary(workspace_dir: Option<String>, user_id: Option<i64>) -> Result<DashboardSummary, String> {
    let db_path = find_db(workspace_dir).ok_or("Daily DB not found".to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // date label
    let date = chrono::Local::now().date_naive().to_string();

    // sessions count and total minutes for today
    let mut stmt = conn.prepare("SELECT COUNT(*), COALESCE(SUM((strftime('%s', COALESCE(end_time, datetime('now'))) - strftime('%s', start_time))/60.0), 0) FROM session WHERE DATE(start_time) = DATE('now')").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let (sessions_count, focus_minutes) = if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        (r.get::<_, i64>(0).unwrap_or(0), r.get::<_, f64>(1).unwrap_or(0.0))
    } else { (0, 0.0) };

    // distractions today
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM distraction WHERE DATE(created_at) = DATE('now')").map_err(|e| e.to_string())?;
    let distractions_count: i64 = stmt.query_row([], |r| r.get(0)).map_err(|e| e.to_string())?;

    // pending goals/tasks
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM goal WHERE COALESCE(completed,0)=0").map_err(|e| e.to_string())?;
    let pending_goals: i64 = stmt.query_row([], |r| r.get(0)).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM task WHERE COALESCE(completed,0)=0").map_err(|e| e.to_string())?;
    let pending_tasks: i64 = stmt.query_row([], |r| r.get(0)).map_err(|e| e.to_string())?;

    // upcoming alarms (limit 10)
    let mut upcoming_alarms = Vec::new();
    if let Some(col) = pick_alarm_time_col(&conn) {
        let sql = format!("SELECT {}, COALESCE(label, '') FROM alarm ORDER BY {} LIMIT 10", col, col);
        if let Ok(mut s) = conn.prepare(&sql) {
            if let Ok(mut rows) = s.query([]) {
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    let label: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    upcoming_alarms.push(DashboardAlarm { time, label });
                }
            }
        }
    }

    // upcoming events
    let mut upcoming_events = Vec::new();
    if let Ok(mut s) = conn.prepare("SELECT id, event_type, event_time, details_json FROM event WHERE event_time >= datetime('now') ORDER BY event_time LIMIT 10") {
        if let Ok(mut rows) = s.query([]) {
            while let Ok(Some(r)) = rows.next() {
                upcoming_events.push(DashboardEvent {
                    id: r.get(0).ok(),
                    event_type: r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default(),
                    event_time: r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default(),
                    details_json: r.get::<_, Option<String>>(3).unwrap_or(None),
                });
            }
        }
    }

    // reminders
    let mut reminders = Vec::new();
    if let Ok(mut s) = conn.prepare("SELECT COALESCE(text, '') FROM reminder ORDER BY COALESCE(created_at, datetime('now')) DESC LIMIT 10") {
        if let Ok(mut rows) = s.query([]) {
            while let Ok(Some(r)) = rows.next() {
                reminders.push(r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default());
            }
        }
    }

    // recent journal entries: call personality DB via backend journals module if user_id supplied
    let mut recent_journal_entries = Vec::new();
    if let Some(uid) = user_id {
        if let Ok(list) = crate::backend::journals::list_journal_entries(uid, Some(5)) {
            for je in list {
                let excerpt = je.content.chars().take(200).collect::<String>();
                recent_journal_entries.push(DashboardJournalEntry { id: je.id, created_at: je.created_at, provider: je.provider, excerpt });
            }
        }
    }

    Ok(DashboardSummary {
        date,
        sessions_count,
        focus_minutes,
        distractions_count,
        pending_goals,
        pending_tasks,
        upcoming_alarms,
        upcoming_events,
        reminders,
        recent_journal_entries,
    })
}

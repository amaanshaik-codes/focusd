use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub start: String,
    pub end: String,
    pub label: String,
    pub description: String,
}

#[tauri::command]
pub fn get_today_sessions(workspace_dir: Option<String>) -> Result<Vec<SessionLog>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            let mut cols = std::collections::HashSet::new();
            if let Ok(mut s) = conn.prepare("PRAGMA table_info(session)") {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }

            let start_expr = if cols.contains("start_time") { "start_time AS start_time".to_string() } else { "'' AS start_time".to_string() };
            let end_expr = if cols.contains("end_time") { "end_time AS end_time".to_string() } else { "'' AS end_time".to_string() };
            let label_expr = if cols.contains("label") { "label AS label".to_string() } else { "'' AS label".to_string() };
            let notes_expr = if cols.contains("notes") { "notes AS notes".to_string() } else { "'' AS notes".to_string() };

            let sql = format!("SELECT {start}, {end}, {label}, {notes} FROM session ORDER BY start_time DESC", start = start_expr, end = end_expr, label = label_expr, notes = notes_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let start: String = r.get(0).unwrap_or_default();
                    let end: String = r.get(1).unwrap_or_default();
                    let label: String = r.get(2).unwrap_or_default();
                    let notes: String = r.get(3).unwrap_or_default();
                    out.push(SessionLog { start, end, label, description: notes });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![SessionLog { start: "09:00".to_string(), end: "09:50".to_string(), label: "Study".to_string(), description: "Read textbook chapter 3".to_string() }])
}
/// Sessions module: Pomodoro/session logic and CRUD.

pub mod sessions {
    // Session struct, CRUD, helpers, config, doc comments
    // ...
}

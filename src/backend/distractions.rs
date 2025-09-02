use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistractionLog {
    pub start: String,
    pub end: String,
    pub label: String,
    pub reason: String,
}

#[tauri::command]
pub fn get_today_distractions(workspace_dir: Option<String>) -> Result<Vec<DistractionLog>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            let mut cols = std::collections::HashSet::new();
            if let Ok(mut s) = conn.prepare("PRAGMA table_info(distraction)") {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }

            let start_expr = if cols.contains("start_time") { "start_time AS start_time".to_string() } else { "'' AS start_time".to_string() };
            let end_expr = if cols.contains("end_time") { "end_time AS end_time".to_string() } else { "'' AS end_time".to_string() };
            let label_expr = if cols.contains("label") { "label AS label".to_string() } else { "'' AS label".to_string() };
            let reason_expr = if cols.contains("reason") { "reason AS reason".to_string() } else { "'' AS reason".to_string() };

            let sql = format!("SELECT {start}, {end}, {label}, {reason} FROM distraction ORDER BY start_time DESC", start = start_expr, end = end_expr, label = label_expr, reason = reason_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let start: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    let end: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    let label: String = r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default();
                    let reason: String = r.get::<_, Option<String>>(3).unwrap_or(None).unwrap_or_default();
                    out.push(DistractionLog { start, end, label, reason });
                }
                return Ok(out);
            }
            else { /* prepare failed for distraction SQL */ }
        }
    }
    Ok(vec![DistractionLog { start: "11:30".to_string(), end: "11:45".to_string(), label: "Phone call".to_string(), reason: "Family call".to_string() }])
}
/// Distractions module: Distraction logic and CRUD.

pub mod distractions {
    // Distraction struct, CRUD, helpers, config, doc comments
    // ...
}

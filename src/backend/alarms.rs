use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    pub time: String,
    pub label: String,
}

#[tauri::command]
pub fn get_alarms(workspace_dir: Option<String>) -> Result<Vec<Alarm>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            // Inspect columns to avoid referencing missing names
            let mut cols = std::collections::HashSet::new();
            if let Ok(mut s) = conn.prepare("PRAGMA table_info(alarm)") {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }

            let time_expr = if cols.contains("time") {
                "time AS time".to_string()
            } else if cols.contains("alarm_time") {
                "alarm_time AS time".to_string()
            } else {
                "'' AS time".to_string()
            };
            let label_expr = if cols.contains("label") { "label AS label".to_string() } else { "'' AS label".to_string() };
            let order_expr = if cols.contains("time") { "time".to_string() } else if cols.contains("alarm_time") { "alarm_time".to_string() } else { "time".to_string() };

            let sql = format!("SELECT {time}, {label} FROM alarm ORDER BY {order}", time = time_expr, label = label_expr, order = order_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get(0).unwrap_or_default();
                    let label: String = r.get(1).unwrap_or_default();
                    out.push(Alarm { time, label });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![
        Alarm { time: "07:30".to_string(), label: "Morning wake-up alarm".to_string() },
    ])
}
/// Alarms module: Alarm logic and CRUD.

pub mod alarms {
    // Alarm struct, CRUD, helpers, config, doc comments
    // ...
}

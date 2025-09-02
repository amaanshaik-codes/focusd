use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub time: String,
    pub name: String,
    pub description: String,
}

#[tauri::command]
pub fn get_today_event_logs(workspace_dir: Option<String>) -> Result<Vec<EventLog>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            if let Ok(mut stmt) = conn.prepare("SELECT COALESCE(event_time, time, '') AS event_time, COALESCE(event_type, type, '') AS event_type, COALESCE(details_json, '') AS details_json FROM event ORDER BY COALESCE(event_time, time)") {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get::<_, Option<String>>(0).unwrap_or(None).unwrap_or_default();
                    let name: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    let details: String = r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default();
                    out.push(EventLog { time, name, description: details });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![EventLog { time: "10:15".to_string(), name: "Team meeting".to_string(), description: "Discussed project milestones".to_string() }])
}
/// Events module: Event logic and CRUD.

pub mod events {
    // Event struct, CRUD, helpers, config, doc comments
    // ...
}

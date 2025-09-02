use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTap {
    pub time: String,
    pub label: String,
}

#[tauri::command]
pub fn get_today_core_card_taps(workspace_dir: Option<String>) -> Result<Vec<CardTap>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            let mut cols = std::collections::HashSet::new();
            if let Ok(mut s) = conn.prepare("PRAGMA table_info(core_card_tap)") {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) { cols.insert(name); }
                    }
                }
            }

            let time_expr = if cols.contains("time") { "time AS time".to_string() } else if cols.contains("tap_time") { "tap_time AS time".to_string() } else { "'' AS time".to_string() };
            let label_expr = if cols.contains("label") { "label AS label".to_string() } else { "'' AS label".to_string() };
            let order_expr = if cols.contains("time") { "time".to_string() } else if cols.contains("tap_time") { "tap_time".to_string() } else { "time".to_string() };

            let sql = format!("SELECT {time}, {label} FROM core_card_tap ORDER BY {order}", time = time_expr, label = label_expr, order = order_expr);
            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let time: String = r.get(0).unwrap_or_default();
                    let label: String = r.get(1).unwrap_or_default();
                    out.push(CardTap { time, label });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![
        CardTap { time: "08:00".to_string(), label: "Wake (core card)".to_string() },
    ])
}
/// Cards module: CRUD and logic for RFID cards.

pub mod cards {
    // Card struct, CRUD, helpers, config, doc comments
    // ...
}

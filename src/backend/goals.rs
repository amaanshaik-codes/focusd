use serde::{Serialize, Deserialize};
use rusqlite::Connection;
use crate::backend::utility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub title: String,
    pub deadline: String,
    pub created: String,
    pub linked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    pub deadline: String,
    pub created: String,
    pub linked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub text: String,
}

#[tauri::command]
pub fn get_pending_goals(workspace_dir: Option<String>) -> Result<Vec<Goal>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            // Inspect table columns to avoid referencing missing columns (which causes prepare errors)
            let mut col_stmt = conn.prepare("PRAGMA table_info(goal)").ok();
            let mut cols = std::collections::HashSet::new();
            if let Some(mut s) = col_stmt {
                if let Ok(mut rows) = s.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(name) = r.get::<_, String>(1) {
                            cols.insert(name);
                        }
                    }
                }
            }

            // Choose column expressions only from available columns, aliasing them consistently
            let title_expr = if cols.contains("title") {
                "title AS title".to_string()
            } else if cols.contains("description") {
                "description AS title".to_string()
            } else {
                "'' AS title".to_string()
            };
            let deadline_expr = if cols.contains("deadline") {
                "deadline AS deadline".to_string()
            } else if cols.contains("target_date") {
                "target_date AS deadline".to_string()
            } else {
                "'' AS deadline".to_string()
            };
            let created_expr = if cols.contains("created_at") {
                "created_at AS created".to_string()
            } else {
                "datetime('now') AS created".to_string()
            };
            let linked_expr = if cols.contains("linked_json") {
                "linked_json AS linked_json".to_string()
            } else {
                "'[]' AS linked_json".to_string()
            };

            let order_expr = if cols.contains("deadline") {
                "deadline".to_string()
            } else if cols.contains("target_date") {
                "target_date".to_string()
            } else {
                "created".to_string()
            };

            let sql = format!(
                "SELECT {title}, {deadline}, {created}, {linked} FROM goal WHERE COALESCE(completed, 0) = 0 ORDER BY {order}",
                title = title_expr,
                deadline = deadline_expr,
                created = created_expr,
                linked = linked_expr,
                order = order_expr
            );

            if let Ok(mut stmt) = conn.prepare(&sql) {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let title: String = r.get(0).unwrap_or_default();
                    let deadline: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    let created: String = r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default();
                    let linked_json: String = r.get::<_, Option<String>>(3).unwrap_or(None).unwrap_or_default();
                    let linked: Vec<String> = serde_json::from_str(&linked_json).unwrap_or_else(|_| vec![]);
                    out.push(Goal { title, deadline, created, linked });
                }
                return Ok(out);
            }
        }
    }
    // fallback demo data
    Ok(vec![Goal {
        title: "Finish reading productivity book".to_string(),
        deadline: "2025-09-10".to_string(),
        created: "2025-08-20".to_string(),
        linked: vec!["Task - Read 10 pages daily".to_string(), "Reminder - Schedule reading time".to_string()],
    }])
}

#[tauri::command]
pub fn get_pending_tasks(workspace_dir: Option<String>) -> Result<Vec<Task>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            // Task table may have similar naming variations
            if let Ok(mut stmt) = conn.prepare("SELECT COALESCE(title, '') AS title, COALESCE(deadline, '') AS deadline, COALESCE(created_at, datetime('now')) AS created_at, COALESCE(linked_json, '[]') AS linked_json FROM task WHERE COALESCE(completed, 0) = 0 ORDER BY COALESCE(deadline, '')") {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let title: String = r.get(0).unwrap_or_default();
                    let deadline: String = r.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default();
                    let created: String = r.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default();
                    let linked_json: String = r.get::<_, Option<String>>(3).unwrap_or(None).unwrap_or_default();
                    let linked: Vec<String> = serde_json::from_str(&linked_json).unwrap_or_else(|_| vec![]);
                    out.push(Task { title, deadline, created, linked });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![Task {
        title: "Reply to client emails".to_string(),
        deadline: "2025-09-03".to_string(),
        created: "2025-09-02".to_string(),
        linked: vec!["Goal - Maintain client relationships".to_string()],
    }])
}

#[tauri::command]
pub fn get_reminders(workspace_dir: Option<String>) -> Result<Vec<Reminder>, String> {
    if let Some(path) = utility::find_daily_db(workspace_dir, None) {
        if let Ok(conn) = Connection::open(path) {
            // Reminder should be stable, but protect against missing created_at
            if let Ok(mut stmt) = conn.prepare("SELECT COALESCE(text, '') AS text FROM reminder ORDER BY COALESCE(created_at, datetime('now')) DESC") {
                let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Ok(Some(r)) = rows.next() {
                    let text: String = r.get(0).unwrap_or_default();
                    out.push(Reminder { text });
                }
                return Ok(out);
            }
        }
    }
    Ok(vec![Reminder { text: "Drink water regularly".to_string() }])
}
/// Goals module: Goal logic and CRUD.

pub mod goals {
    // Goal struct, CRUD, helpers, config, doc comments
    // ...
}

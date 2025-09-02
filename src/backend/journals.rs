use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};
use crate::backend::personality_db::PERSONALITY_DB_PATH;

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i64,
    pub user_id: i64,
    pub created_at: String,
    pub provider: String,
    pub model: Option<String>,
    pub content: String,
    pub tokens: Option<i64>,
}

/// Ensure journals table exists
#[tauri::command]
pub fn init_journals_table() -> Result<(), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS journal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT,
            content TEXT NOT NULL,
            tokens INTEGER DEFAULT NULL
        );
    "#).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_journal_entry(user_id: i64, provider: String, model: Option<String>, content: String, tokens: Option<i64>) -> Result<i64, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO journal_entries (user_id, created_at, provider, model, content, tokens) VALUES (?, datetime('now'), ?, ?, ?, ?)",
        params![user_id, provider, model, content, tokens],
    ).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(id)
}

#[tauri::command]
pub fn list_journal_entries(user_id: i64, limit: Option<i64>) -> Result<Vec<JournalEntry>, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(50);
    let mut stmt = conn.prepare("SELECT id, user_id, created_at, provider, model, content, tokens FROM journal_entries WHERE user_id = ? ORDER BY created_at DESC LIMIT ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id, lim]).map_err(|e| e.to_string())?;
    let mut res = Vec::new();
    while let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = r.get(0).map_err(|e| e.to_string())?;
        let user_id: i64 = r.get(1).map_err(|e| e.to_string())?;
        let created_at: String = r.get(2).map_err(|e| e.to_string())?;
        let provider: String = r.get(3).map_err(|e| e.to_string())?;
        let model: Option<String> = r.get(4).map_err(|e| e.to_string())?;
        let content: String = r.get(5).map_err(|e| e.to_string())?;
        let tokens: Option<i64> = r.get(6).map_err(|e| e.to_string())?;
        res.push(JournalEntry { id, user_id, created_at, provider, model, content, tokens });
    }
    Ok(res)
}

#[tauri::command]
pub fn get_journal_entry(entry_id: i64) -> Result<Option<JournalEntry>, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, user_id, created_at, provider, model, content, tokens FROM journal_entries WHERE id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![entry_id]).map_err(|e| e.to_string())?;
    if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = r.get(0).map_err(|e| e.to_string())?;
        let user_id: i64 = r.get(1).map_err(|e| e.to_string())?;
        let created_at: String = r.get(2).map_err(|e| e.to_string())?;
        let provider: String = r.get(3).map_err(|e| e.to_string())?;
        let model: Option<String> = r.get(4).map_err(|e| e.to_string())?;
        let content: String = r.get(5).map_err(|e| e.to_string())?;
        let tokens: Option<i64> = r.get(6).map_err(|e| e.to_string())?;
        Ok(Some(JournalEntry { id, user_id, created_at, provider, model, content, tokens }))
    } else {
        Ok(None)
    }
}

// Helper: compute day count for a user from journal entries
pub fn compute_day_count(user_id: i64) -> Result<i64, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM journal_entries WHERE user_id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let cnt: i64 = r.get(0).map_err(|e| e.to_string())?;
        Ok(cnt + 1) // next day count
    } else {
        Ok(1)
    }
}

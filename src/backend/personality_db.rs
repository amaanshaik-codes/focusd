/// Fetch all-time stats (goals accomplished, total session hours, avg sleep hours)
#[tauri::command]
pub fn get_all_time_stats() -> Result<(i64, f64, f64), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT goals_accomplished, total_session_hours, avg_sleep_hours FROM all_time_stats WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok((row.get(0).unwrap_or(0), row.get(1).unwrap_or(0.0), row.get(2).unwrap_or(0.0)))
    } else {
        Ok((0, 0.0, 0.0))
    }
}
/// Productivity personality types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersonalityType {
    Planner,
    Sprinter,
    DeepWorker,
    Multitasker,
    Collaborator,
    Minimalist,
    Optimizer,
    Reflector,
    Unknown,
}

/// Infer personality type from onboarding answers (simple rules-based for demo)
#[tauri::command]
pub fn infer_personality_type(answers: &[i32], _challenge: &str) -> PersonalityType {
    // Example logic: (real logic can be more sophisticated)
    let plan = answers.get(0).copied().unwrap_or(3);
    let multi = answers.get(3).copied().unwrap_or(3);
    let collab = answers.get(8).copied().unwrap_or(3);
    if plan >= 4 && multi <= 2 {
        PersonalityType::Planner
    } else if multi >= 4 {
        PersonalityType::Multitasker
    } else if collab >= 4 {
        PersonalityType::Collaborator
    } else {
        PersonalityType::Unknown
    }
}

/// Save inferred personality type
#[tauri::command]
pub fn save_personality_type(ptype: &PersonalityType) -> Result<(), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO personality_profile (id, personality_type, inferred_at) VALUES (1, ?, datetime('now'))",
        params![format!("{:?}", ptype)],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Update all-time stats by writing the provided values (or use aggregation helpers).
#[tauri::command]
pub fn update_all_time_stats(goals: i64, session_hours: f64, avg_sleep: f64) -> Result<(), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO all_time_stats (id, goals_accomplished, total_session_hours, avg_sleep_hours, updated_at) VALUES (1, ?, ?, ?, datetime('now'))",
        params![goals, session_hours, avg_sleep],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Aggregate across daily workspace DB files and update `all_time_stats`.
/// Assumptions: daily DB files are located in the provided `workspace_dir` or current directory
/// and are named like `focusd_YYYY-MM-DD.sqlite3` or `focusd_YYYYMMDD.*`.
#[tauri::command]
pub fn aggregate_all_time_stats(workspace_dir: Option<String>) -> Result<(i64, f64, f64), String> {
    use std::fs;
    use std::path::Path;

    let dir = workspace_dir.unwrap_or_else(|| "./".to_string());
    let mut total_goals = 0i64;
    let mut total_session_hours = 0.0f64;
    let mut sleep_hours_accum: Vec<f64> = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| format!("failed to read workspace dir {}: {}", dir, e))?;
    for e in entries {
        if let Ok(entry) = e {
            let path = entry.path();
            if !path.is_file() { continue; }
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if !fname.starts_with("focusd_") { continue; }
                if !(fname.ends_with(".sqlite3") || fname.ends_with(".sqlite") || fname.ends_with(".db")) { continue; }
                // open daily DB
                if let Ok(conn) = Connection::open(&path) {
                    // goals completed count (if table exists)
                    if let Ok(mut stmt) = conn.prepare("SELECT COUNT(*) FROM goal WHERE completed = 1") {
                        if let Ok(mut rows) = stmt.query([]) {
                            if let Ok(Some(r)) = rows.next() {
                                let cnt: i64 = r.get(0).unwrap_or(0);
                                total_goals += cnt;
                            }
                        }
                    }
                    // session hours sum
                    if let Ok(mut stmt) = conn.prepare("SELECT SUM((julianday(end_time) - julianday(start_time)) * 24.0) FROM session WHERE end_time IS NOT NULL") {
                        if let Ok(mut rows) = stmt.query([]) {
                            if let Ok(Some(r)) = rows.next() {
                                let hours: f64 = r.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0);
                                total_session_hours += hours;
                            }
                        }
                    }
                    // sleep detection: look for sessions/events mentioning 'sleep' with durations
                    if let Ok(mut stmt) = conn.prepare("SELECT (julianday(end_time) - julianday(start_time)) * 24.0 FROM session WHERE end_time IS NOT NULL AND (LOWER(notes) LIKE '%sleep%' OR LOWER(ai_summary) LIKE '%sleep%')") {
                        if let Ok(mut rows) = stmt.query([]) {
                            while let Ok(Some(r)) = rows.next() {
                                let h: f64 = r.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0);
                                if h > 0.0 { sleep_hours_accum.push(h); }
                            }
                        }
                    }
                }
            }
        }
    }

    // Prefer an explicit `sleep` table if present in any daily DB
    let mut avg_sleep = 0.0f64;
    let mut found_sleep_table = false;
    let entries = fs::read_dir(&dir).map_err(|e| format!("failed to read workspace dir {}: {}", dir, e))?;
    for e in entries {
        if let Ok(entry) = e {
            let path = entry.path();
            if !path.is_file() { continue; }
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if !fname.starts_with("focusd_") { continue; }
                if !(fname.ends_with(".sqlite3") || fname.ends_with(".sqlite") || fname.ends_with(".db")) { continue; }
                if let Ok(conn) = Connection::open(&path) {
                    // Check for a sleep table
                    if let Ok(mut check_stmt) = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='sleep' LIMIT 1") {
                        if let Ok(mut rows) = check_stmt.query([]) {
                            if let Ok(Some(_r)) = rows.next() {
                                found_sleep_table = true;
                                // Try common column patterns
                                let try_cols = ["duration_hours", "duration", "hours"]; 
                                let mut got: Option<f64> = None;
                                for col in &try_cols {
                                    let q = format!("SELECT AVG({}) FROM sleep", col);
                                    if let Ok(mut stmt2) = conn.prepare(&q) {
                                        if let Ok(mut rows2) = stmt2.query([]) {
                                            if let Ok(Some(r2)) = rows2.next() {
                                                let v: f64 = r2.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0);
                                                if v > 0.0 { got = Some(v); break; }
                                            }
                                        }
                                    }
                                }
                                // If no duration column, try start/end times
                                if got.is_none() {
                                    if let Ok(mut stmt3) = conn.prepare("SELECT AVG((julianday(end_time) - julianday(start_time)) * 24.0) FROM sleep WHERE end_time IS NOT NULL AND start_time IS NOT NULL") {
                                        if let Ok(mut rows3) = stmt3.query([]) {
                                            if let Ok(Some(r3)) = rows3.next() {
                                                let v: f64 = r3.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0);
                                                if v > 0.0 { got = Some(v); }
                                            }
                                        }
                                    }
                                }
                                if let Some(v) = got { avg_sleep = v; break; }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found_sleep_table {
        avg_sleep = if sleep_hours_accum.is_empty() {
            // fallback to existing stored value if present
            if let Ok((_, _, stored_sleep)) = get_all_time_stats() { stored_sleep } else { 0.0 }
        } else {
            sleep_hours_accum.iter().sum::<f64>() / sleep_hours_accum.len() as f64
        };
    }

    // Persist aggregated results
    update_all_time_stats(total_goals, total_session_hours, avg_sleep)?;
    Ok((total_goals, total_session_hours, avg_sleep))
}

/// Generate an AI prompt based on profile, stats, and answers
#[tauri::command]
pub fn generate_ai_prompt() -> Result<String, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    // Get answers
    let answers: Vec<i32> = get_onboarding_answers()?;
    // Get personality type
    let mut stmt = conn.prepare("SELECT personality_type FROM personality_profile WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let ptype: String = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        row.get(0).unwrap_or("Unknown".to_string())
    } else { "Unknown".to_string() };
    // Get stats
    let mut stmt = conn.prepare("SELECT goals_accomplished, total_session_hours, avg_sleep_hours FROM all_time_stats WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let (goals, hours, sleep) = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        (row.get(0).unwrap_or(0), row.get(1).unwrap_or(0.0), row.get(2).unwrap_or(0.0))
    } else { (0, 0.0, 0.0) };
    // Build prompt
    let prompt = format!(
        "You are an AI productivity assistant.\n\nUser profile:\n- Personality type: {ptype}\n- Onboarding answers: {answers:?}\n- Goals accomplished: {goals}\n- Total session hours: {hours:.1}\n- Average sleep hours: {sleep:.1}\n\nContext: Use this information to tailor your advice and responses.\n\nTask: Help the user plan their day and overcome their biggest productivity challenge."
    );
    Ok(prompt)
}
/// Personality DB schema and onboarding answer storage for Focusd.
/// This module manages the persistent user profile, onboarding answers, inferred personality type, and all-time stats.

use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};

/// Path to the persistent personality database (not rotated daily)
pub const PERSONALITY_DB_PATH: &str = "focusd_personality.db";

/// Create the personality DB and tables if they do not exist.
#[tauri::command]
pub fn init_personality_db() -> Result<(), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS onboarding_answers (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            answers_json TEXT NOT NULL,
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS personality_profile (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            personality_type TEXT,
            inferred_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS all_time_stats (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            goals_accomplished INTEGER DEFAULT 0,
            total_session_hours REAL DEFAULT 0.0,
            avg_sleep_hours REAL DEFAULT 0.0,
            updated_at TEXT DEFAULT (datetime('now'))
        );
    "#).map_err(|e| e.to_string())?;
    Ok(())
}

/// Save onboarding answers (as JSON array of answers)
#[tauri::command]
pub fn save_onboarding_answers(answers: &[i32], _challenge: &str) -> Result<(), String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let answers_json = serde_json::to_string(answers).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO onboarding_answers (id, answers_json, updated_at) VALUES (1, ?, datetime('now'))",
        params![answers_json],
    ).map_err(|e| e.to_string())?;
    // Save challenge answer as part of the profile (optional, can be extended)
    conn.execute(
        "INSERT OR REPLACE INTO personality_profile (id, personality_type, inferred_at) VALUES (1, NULL, datetime('now'))",
        [],
    ).map_err(|e| e.to_string())?;
    // Auto-infer personality type and persist it
    let ptype = infer_personality_type(answers, "" );
    let _ = save_personality_type(&ptype);
    Ok(())
}

/// Retrieve onboarding answers
#[tauri::command]
pub fn get_onboarding_answers() -> Result<Vec<i32>, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT answers_json FROM onboarding_answers WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let json: String = row.get(0).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    } else {
        Ok(vec![])
    }
}

/// Return persisted personality type string if available
#[tauri::command]
pub fn get_personality_type() -> Result<Option<String>, String> {
    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT personality_type FROM personality_profile WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(row.get(0).map_err(|e| e.to_string())?)
    } else { Ok(None) }
}

/// Combined profile + stats return type for convenience
#[derive(Serialize, Deserialize)]
pub struct ProfileStats {
    pub personality_type: Option<String>,
    pub goals_completed_all_time: i64,
    pub total_session_hours_all_time: f64,
    pub avg_sleep_hours_all_time: f64,
}

/// Synchronous helper that returns the personality type and aggregated stats.
pub fn get_profile_and_stats(workspace_dir: Option<String>) -> Result<ProfileStats, String> {
    let p = get_personality_type()?;
    let (goals, hours, sleep) = aggregate_all_time_stats(workspace_dir)?;
    Ok(ProfileStats {
        personality_type: p,
        goals_completed_all_time: goals,
        total_session_hours_all_time: hours,
        avg_sleep_hours_all_time: sleep,
    })
}

/// Tauri command (async) wrapper to retrieve profile and stats in one call.
#[tauri::command]
pub async fn get_profile_and_stats_async(workspace_dir: Option<String>) -> Result<ProfileStats, String> {
    let wd = workspace_dir.clone();
    tokio::task::spawn_blocking(move || get_profile_and_stats(wd)).await.map_err(|e| e.to_string())?.map_err(|e| e)
}

// Async wrappers that use spawn_blocking so these can be called safely from async handlers / Tauri
#[tauri::command]
pub async fn get_personality_type_async() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(|| get_personality_type()).await.map_err(|e| e.to_string())?.map_err(|e| e)
}

#[tauri::command]
pub async fn save_onboarding_answers_async(answers: Vec<i32>, challenge: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_onboarding_answers(&answers.iter().map(|i| *i).collect::<Vec<i32>>(), &challenge)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn aggregate_all_time_stats_async(workspace_dir: Option<String>) -> Result<(i64, f64, f64), String> {
    tokio::task::spawn_blocking(move || aggregate_all_time_stats(workspace_dir)).await.map_err(|e| e.to_string())?.map_err(|e| e)
}

#[tauri::command]
pub async fn update_all_time_stats_async(goals: i64, session_hours: f64, avg_sleep: f64) -> Result<(), String> {
    tokio::task::spawn_blocking(move || update_all_time_stats(goals, session_hours, avg_sleep)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn init_personality_db_async() -> Result<(), String> {
    tokio::task::spawn_blocking(|| init_personality_db()).await.map_err(|e| e.to_string())?
}

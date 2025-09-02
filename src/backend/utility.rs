use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use chrono::NaiveDate;

#[allow(dead_code)]
/// Resets all user data in the database (for development/troubleshooting)
pub fn reset_database(db_path: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let tables = [
        "user", "card", "session", "event", "distraction", "goal", "alarm", "log", "user_setting", "audit_log"
    ];
    for table in &tables {
        let sql = format!("DELETE FROM {}", table);
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(dead_code)]
/// Health check for database connectivity
pub fn health_check_database(db_path: &str) -> Result<(), String> {
    Connection::open(db_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(dead_code)]
/// Health check for AI connectivity (dummy, extend for real API)
pub fn health_check_ai(api_key: &str) -> Result<(), String> {
    if api_key.is_empty() {
        Err("API key not set".to_string())
    } else {
        Ok(())
    }
}

#[allow(dead_code)]
/// Error logging utility (append to log file)
pub fn log_error(context: &str, error: &str) {
    let log_line = format!("[ERROR] [{}] {}\n", context, error);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("focusd_error.log")
        .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));
}

/// Find the daily rotating DB file for a given date (defaults to today).
/// Searches `workspace_dir` (or current dir) for files starting with `focusd_<date>`
/// where <date> can be YYYY-MM-DD or YYYYMMDD and extensions .sqlite3/.sqlite/.db
pub fn find_daily_db(workspace_dir: Option<String>, date: Option<NaiveDate>) -> Option<PathBuf> {
    let dir = workspace_dir.unwrap_or_else(|| "./".to_string());
    let d = date.unwrap_or_else(|| chrono::Local::now().date_naive());
    // Support multiple naming schemes and be permissive when matching files
    let prefixes = vec![format!("focusd_{}", d.format("%Y-%m-%d")), format!("focusd_{}", d.format("%Y%m%d"))];
    let exts = [".sqlite3", ".sqlite", ".db"];
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if !path.is_file() { continue; }
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                // Exact or starts_with match (original behavior)
                for p in &prefixes {
                    for ext in &exts {
                        let cand = format!("{}{}", p, ext);
                        if fname == cand || (fname.starts_with(p) && fname.ends_with(ext)) {
                            return Some(path.clone());
                        }
                    }
                }

                // More permissive: sometimes temp files or alternate naming include the date
                // in different positions or have extra suffixes; check contains(date) + ext.
                let date1 = d.format("%Y-%m-%d").to_string();
                let date2 = d.format("%Y%m%d").to_string();
                for ext in &exts {
                    if fname.ends_with(ext) && (fname.contains(&date1) || fname.contains(&date2) || fname.contains("focusd_")) {
                        return Some(path.clone());
                    }
                }
            }
        }
    }
    None
}

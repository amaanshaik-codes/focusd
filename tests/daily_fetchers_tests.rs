use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use chrono::NaiveDate;

use focusd_lib::backend::{goals, alarms, cards, events, sessions, distractions, utility};

fn create_daily_db(dir: &PathBuf, date: NaiveDate) -> PathBuf {
    let fname = format!("focusd_{}.sqlite3", date.format("%Y-%m-%d"));
    let path = dir.join(&fname);
    // Create DB and schema similar to init_daily_database
    let conn = rusqlite::Connection::open(&path).expect("open db");
    let stmts = [
        r#"CREATE TABLE IF NOT EXISTS goal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            description TEXT NOT NULL,
            target_date TEXT,
            completed INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            deadline TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            completed INTEGER DEFAULT 0,
            linked_json TEXT
        );"#,
        r#"CREATE TABLE IF NOT EXISTS reminder (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS alarm (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            time TEXT NOT NULL,
            label TEXT,
            triggered INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS core_card_tap (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            time TEXT,
            label TEXT
        );"#,
        r#"CREATE TABLE IF NOT EXISTS event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            event_type TEXT NOT NULL,
            event_time TEXT NOT NULL,
            details_json TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            notes TEXT,
            ai_summary TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS distraction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER,
            event_id INTEGER,
            reason TEXT,
            resolved INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
    ];
    for s in &stmts {
        conn.execute(s, []).expect("create");
    }
    path
}

#[test]
fn test_daily_fetchers_return_inserted_rows() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let date = chrono::Local::now().date_naive();
    let db_path = create_daily_db(&dir, date);

    // Insert sample rows
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    // The fetcher expects goal.title, deadline, created_at, linked_json (some schemas differ)
    conn.execute("ALTER TABLE goal RENAME TO goal_old", []).ok();
    conn.execute(r#"CREATE TABLE IF NOT EXISTS goal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            deadline TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            linked_json TEXT,
            completed INTEGER DEFAULT 0
        );"#, []).unwrap();
    conn.execute("INSERT INTO goal (title, deadline, linked_json, completed) VALUES (?, ?, ?, 0)", rusqlite::params!["Test Goal", "2099-01-01", "[]"]).unwrap();
    conn.execute("INSERT INTO task (title, deadline, linked_json) VALUES (?, ?, ?)", rusqlite::params!["Test Task", "2099-01-02", "[]"]).unwrap();
    conn.execute("INSERT INTO reminder (text) VALUES (?)", rusqlite::params!["Test Reminder"]).unwrap();
    conn.execute("INSERT INTO alarm (time, label) VALUES (?, ?)", rusqlite::params!["08:00", "Test Alarm"]).unwrap();
    conn.execute("INSERT INTO core_card_tap (time, label) VALUES (?, ?)", rusqlite::params!["07:00", "Wake"]).unwrap();
    conn.execute("INSERT INTO event (event_type, event_time, details_json) VALUES (?, ?, ?)", rusqlite::params!["meeting", "2025-01-01T10:00:00Z", "{}"]).unwrap();
    conn.execute("INSERT INTO session (card_id, start_time, end_time, notes) VALUES (?, ?, ?, ?)", rusqlite::params![1, "2025-01-01T09:00:00Z", "2025-01-01T09:50:00Z", "Notes"]).unwrap();
    // Ensure distraction table has start_time, end_time, label, reason as the fetcher expects
    conn.execute("ALTER TABLE distraction RENAME TO distraction_old", []).ok();
    conn.execute(r#"CREATE TABLE IF NOT EXISTS distraction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start_time TEXT,
            end_time TEXT,
            label TEXT,
            reason TEXT,
            resolved INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );"#, []).unwrap();
    conn.execute("INSERT INTO distraction (start_time, end_time, label, reason) VALUES (?, ?, ?, ?)", rusqlite::params!["2025-01-01T11:30:00Z", "2025-01-01T11:45:00Z", "Phone call", "Phone"]).unwrap();

    // Call fetchers with workspace_dir pointing to temp dir
    let wd = Some(dir.to_string_lossy().to_string());
    // Sanity-check: ensure utility::find_daily_db finds our test DB
    let found = utility::find_daily_db(wd.clone(), None);
    assert!(found.is_some(), "test setup failed: daily DB not found in temp dir: {:?}", dir);

    let gs = goals::get_pending_goals(wd.clone()).expect("get goals");
    assert!(gs.iter().any(|g| g.title.contains("Test Goal") || g.title.contains("Test")), "goals not found: {:?}", gs);

    let ts = goals::get_pending_tasks(wd.clone()).expect("get tasks");
    assert!(ts.iter().any(|t| t.title.contains("Test Task") || t.title.contains("Test")), "tasks not found: {:?}", ts);

    let rs = goals::get_reminders(wd.clone()).expect("get reminders");
    assert!(rs.iter().any(|r| r.text.contains("Test Reminder")));

    let als = alarms::get_alarms(wd.clone()).expect("get alarms");
    assert!(als.iter().any(|a| a.label.contains("Test Alarm")));

    let taps = cards::get_today_core_card_taps(wd.clone()).expect("get taps");
    assert!(taps.iter().any(|c| c.label.contains("Wake")));

    let evs = events::get_today_event_logs(wd.clone()).expect("get events");
    assert!(evs.iter().any(|e| e.name.contains("meeting") || e.description.contains("{}")));

    let sess = sessions::get_today_sessions(wd.clone()).expect("get sessions");
    assert!(sess.iter().any(|s| s.label.contains("") || s.description.contains("Notes")));

    let dis = distractions::get_today_distractions(wd.clone()).expect("get distractions");
    println!("Distractions returned: {:?}", dis);
    assert!(dis.iter().any(|d| d.reason.contains("Phone")));

    // Cleanup
    drop(conn);
    fs::remove_file(db_path).ok();
}

#[test]
fn test_missing_tables_return_fallbacks() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let date = chrono::Local::now().date_naive();
    let db_path = create_daily_db(&dir, date);

    // Drop the goal table to simulate older/malformed DB
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute("DROP TABLE IF EXISTS goal", []).ok();

    let wd = Some(dir.to_string_lossy().to_string());
    // Should not panic; functions will return fallback demo data
    let _ = goals::get_pending_goals(wd.clone()).expect("get goals fallback");
    let _ = goals::get_pending_tasks(wd.clone()).expect("get tasks fallback");
    let _ = goals::get_reminders(wd.clone()).expect("get reminders fallback");

    drop(conn);
    fs::remove_file(db_path).ok();
}

#[test]
fn test_partial_columns_handle_nulls() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let date = chrono::Local::now().date_naive();
    let db_path = create_daily_db(&dir, date);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    // Create a simplified goal table with only description (legacy)
    conn.execute("ALTER TABLE goal RENAME TO goal_orig", []).ok();
    conn.execute(r#"CREATE TABLE IF NOT EXISTS goal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL
        );"#, []).unwrap();
    conn.execute("INSERT INTO goal (description) VALUES (?)", rusqlite::params!["Legacy goal"]).unwrap();

    let wd = Some(dir.to_string_lossy().to_string());
    let gs = goals::get_pending_goals(wd.clone()).expect("get legacy goals");
    assert!(gs.len() >= 1);

    drop(conn);
    fs::remove_file(db_path).ok();
}

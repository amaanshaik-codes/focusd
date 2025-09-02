use std::fs;
use std::path::PathBuf;
use chrono::NaiveDate;
use rusqlite::Connection;

fn make_daily_db_for(date: NaiveDate, dir: &PathBuf) -> PathBuf {
    let fname = format!("focusd_{}.sqlite3", date.format("%Y-%m-%d"));
    let path = dir.join(fname);
    let _ = fs::remove_file(&path);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            event_time TEXT NOT NULL,
            details_json TEXT
        );
        CREATE TABLE IF NOT EXISTS alarm (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alarm_time TEXT NOT NULL,
            label TEXT
        );
        CREATE TABLE IF NOT EXISTS reminder (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT,
            created_at TEXT
        );
    "#).unwrap();
    path
}

#[test]
fn test_orchestrator_multi_day_aggregation() {
    let tmp = std::env::temp_dir().join("focusd_test_orch");
    let _ = fs::create_dir_all(&tmp);
    // Dates: today and today+1
    let today = chrono::Local::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let p1 = make_daily_db_for(today, &tmp);
    let p2 = make_daily_db_for(tomorrow, &tmp);

    // Insert one event into each DB
    let conn1 = Connection::open(&p1).unwrap();
    conn1.execute("INSERT INTO event (event_type, event_time, details_json) VALUES (?1, ?2, ?3)", rusqlite::params!["meeting", today.to_string(), "{}" ]).unwrap();
    let conn2 = Connection::open(&p2).unwrap();
    conn2.execute("INSERT INTO event (event_type, event_time, details_json) VALUES (?1, ?2, ?3)", rusqlite::params!["standup", tomorrow.to_string(), "{}" ]).unwrap();

    // Call the orchestrator command
    let start = today.to_string();
    let end = tomorrow.to_string();
    // Temporarily change cwd to the temp dir so find_daily_db finds our files
    let cur = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    // Debug: check whether utility finds both DBs
    let found_today = focusd_lib::backend::utility::find_daily_db(Some(".".to_string()), Some(today));
    let found_tomorrow = focusd_lib::backend::utility::find_daily_db(Some(".".to_string()), Some(tomorrow));
    println!("found_today={:?}", found_today.map(|p| p.to_string_lossy().to_string()));
    println!("found_tomorrow={:?}", found_tomorrow.map(|p| p.to_string_lossy().to_string()));
    let res = focusd_lib::backend::orchestrator::get_calendar_range(Some(".".to_string()), start.clone(), end.clone());
    // restore cwd
    std::env::set_current_dir(cur).unwrap();

    assert!(res.is_ok());
    let items = res.unwrap();
    println!("Returned items: {}", items.len());
    for it in &items {
        println!("kind={} time={} title={} details={:?}", it.kind, it.time, it.title, it.details);
    }
    // We expect at least two events aggregated across days
    let event_kinds: Vec<String> = items.iter().filter(|i| i.kind == "event").map(|i| i.title.clone()).collect();
    assert!(event_kinds.iter().any(|t| t == "meeting"));
    assert!(event_kinds.iter().any(|t| t == "standup"));
}

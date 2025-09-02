use tempfile::tempdir;
use std::fs;
use std::path::PathBuf;

use focusd_lib::{test_api, Session, Event, Card};

fn create_db(dir: &PathBuf) -> PathBuf {
    let path = dir.join("test_sessions_events.sqlite3");
    let conn = rusqlite::Connection::open(&path).expect("open db");
    let schema = [
        r#"CREATE TABLE IF NOT EXISTS card (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rfid TEXT NOT NULL,
            type TEXT NOT NULL,
            label TEXT,
            color TEXT,
            metadata_json TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            UNIQUE(rfid, type)
        );"#,
        r#"CREATE TABLE IF NOT EXISTS session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            notes TEXT,
            ai_summary TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );"#,
        r#"CREATE TABLE IF NOT EXISTS event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            event_type TEXT NOT NULL,
            event_time TEXT NOT NULL,
            details_json TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );"#,
    ];
    for s in &schema { conn.execute(s, []).expect("create table"); }
    path
}

#[test]
fn test_session_event_crud_flow() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let db_path = create_db(&dir);
    let db_str = db_path.to_string_lossy().to_string();

    // Create a card to reference
    let card = Card { id: None, rfid: "C1".to_string(), type_: "session".to_string(), label: Some("Card1".to_string()), color: None, metadata_json: None, created_at: None, updated_at: None };
    let card_id = test_api::create_card(db_str.clone(), card).expect("create card");

    // Create a session
    let session = Session { id: None, card_id: card_id as i64, start_time: "2025-09-03T09:00:00Z".to_string(), end_time: Some("2025-09-03T09:30:00Z".to_string()), notes: Some("Testing session".to_string()), ai_summary: None, created_at: None, updated_at: None };
    let sid = test_api::create_session(db_str.clone(), session).expect("create session");

    // Get sessions and assert
    let sessions = test_api::get_sessions(db_str.clone()).expect("get sessions");
    assert!(sessions.iter().any(|s| s.id == Some(sid)));

    // Update session notes
    let mut s = sessions.into_iter().find(|s| s.id == Some(sid)).unwrap();
    s.notes = Some("Updated notes".to_string());
    test_api::update_session(db_str.clone(), s).expect("update session");
    let sessions2 = test_api::get_sessions(db_str.clone()).expect("get sessions after update");
    assert!(sessions2.iter().any(|s| s.id == Some(sid) && s.notes.as_deref() == Some("Updated notes")));

    // Create an event linked to card
    let event = Event { id: None, card_id: Some(card_id as i64), event_type: "meeting".to_string(), event_time: "2025-09-03T10:00:00Z".to_string(), details_json: Some("{}".to_string()), created_at: None };
    let eid = test_api::create_event(db_str.clone(), event).expect("create event");
    let events = test_api::get_events(db_str.clone()).expect("get events");
    assert!(events.iter().any(|e| e.id == Some(eid) && e.event_type == "meeting"));

    // Update event
    let mut ev = events.into_iter().find(|e| e.id == Some(eid)).unwrap();
    ev.details_json = Some("{\"note\":\"updated\"}".to_string());
    test_api::update_event(db_str.clone(), ev).expect("update event");
    let events2 = test_api::get_events(db_str.clone()).expect("get events after update");
    assert!(events2.iter().any(|e| e.id == Some(eid) && e.details_json.as_deref().map(|s| s.contains("updated")).unwrap_or(false)));

    // Delete event and session
    test_api::delete_event(db_str.clone(), eid).expect("delete event");
    test_api::delete_session(db_str.clone(), sid).expect("delete session");
    let events_final = test_api::get_events(db_str.clone()).expect("get events final");
    let sessions_final = test_api::get_sessions(db_str.clone()).expect("get sessions final");
    assert!(!events_final.iter().any(|e| e.id == Some(eid)));
    assert!(!sessions_final.iter().any(|s| s.id == Some(sid)));

    // Cleanup
    fs::remove_file(db_path).ok();
}

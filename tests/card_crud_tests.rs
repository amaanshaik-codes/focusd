use tempfile::tempdir;
use std::fs;
use std::path::PathBuf;

use focusd_lib::{Card, test_api};

fn create_cards_db(dir: &PathBuf) -> PathBuf {
    let path = dir.join("test_cards.sqlite3");
    let conn = rusqlite::Connection::open(&path).expect("open db");
    let stmt = r#"CREATE TABLE IF NOT EXISTS card (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        rfid TEXT NOT NULL,
        type TEXT NOT NULL,
        label TEXT,
        color TEXT,
        metadata_json TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        UNIQUE(rfid, type)
    );"#;
    conn.execute(stmt, []).expect("create card table");
    path
}

#[test]
fn test_card_crud_and_reassign_flows() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let db_path = create_cards_db(&dir);
    let db_str = db_path.to_string_lossy().to_string();

    // Create core card
    let core = Card {
        id: None,
        rfid: "CORE1".to_string(),
        type_: "core".to_string(),
        label: Some("Core Card".to_string()),
        color: None,
        metadata_json: None,
        created_at: None,
        updated_at: None,
    };
    let id1 = test_api::create_card(db_str.clone(), core).expect("create core card");

    // Creating another core should fail
    let core2 = Card { id: None, rfid: "CORE2".to_string(), type_: "core".to_string(), label: None, color: None, metadata_json: None, created_at: None, updated_at: None };
    assert!(test_api::create_card(db_str.clone(), core2).is_err(), "should not allow second core card");

    // Get cards and ensure core exists
    let cards = test_api::get_cards(db_str.clone()).expect("get cards");
    assert!(cards.iter().any(|c| c.id == Some(id1) && c.rfid == "CORE1"));

    // Update core card label
    let mut updated = cards.into_iter().find(|c| c.id == Some(id1)).unwrap();
    updated.label = Some("Core Updated".to_string());
    test_api::update_card(db_str.clone(), updated).expect("update card");
    let cards2 = test_api::get_cards(db_str.clone()).expect("get cards after update");
    assert!(cards2.iter().any(|c| c.id == Some(id1) && c.label.as_deref() == Some("Core Updated")));

    // Create a session card
    let session_card = Card { id: None, rfid: "S1".to_string(), type_: "session".to_string(), label: Some("Session".to_string()), color: None, metadata_json: None, created_at: None, updated_at: None };
    let id2 = test_api::create_card(db_str.clone(), session_card).expect("create session card");

    // Reassign RFID to an already-used RFID should fail
    assert!(test_api::reassign_card_rfid(db_str.clone(), id2, "CORE1".to_string()).is_err(), "reassign to existing RFID should fail");

    // Reassign RFID successfully
    test_api::reassign_card_rfid(db_str.clone(), id2, "S1_NEW".to_string()).expect("reassign success");
    let cards3 = test_api::get_cards(db_str.clone()).expect("get cards after reassign");
    assert!(cards3.iter().any(|c| c.id == Some(id2) && c.rfid == "S1_NEW"));

    // Delete session card
    test_api::delete_card(db_str.clone(), id2).expect("delete session card");
    let cards4 = test_api::get_cards(db_str.clone()).expect("get cards after delete");
    assert!(!cards4.iter().any(|c| c.id == Some(id2)));

    // Delete core card
    test_api::delete_card(db_str.clone(), id1).expect("delete core card");
    let cards5 = test_api::get_cards(db_str.clone()).expect("get cards final");
    assert!(!cards5.iter().any(|c| c.id == Some(id1)));

    // Cleanup
    fs::remove_file(db_path).ok();
}

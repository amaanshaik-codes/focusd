
//! Migration/versioning module: handles all schema upgrades and persistent data versioning.


use rusqlite::{Connection, params};
use std::fmt::Write as _;

#[allow(dead_code)]
/// Run all pending migrations and return migration status.
///
/// - Tracks schema version in `schema_version` table.
/// - Applies migrations stepwise, logs each.
/// - Idempotent and extensible for future upgrades.
/// - Returns a detailed status string.
pub fn run_migrations(db_path: &str) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("DB open error: {e}"))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL, applied_at TEXT NOT NULL)",
        [],
    ).map_err(|e| format!("Create schema_version table error: {e}"))?;

    // Get current version
    let mut stmt = conn.prepare("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let current_version: i64 = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        row.get(0).unwrap_or(0)
    } else {
        0
    };

    // List of migrations: (version, SQL, description)
    let migrations: &[(i64, &str, &str)] = &[
        // Example: (1, "CREATE TABLE ...", "Initial schema")
        (1, "CREATE TABLE IF NOT EXISTS example_table (id INTEGER PRIMARY KEY, name TEXT)", "Initial example table"),
        // Add more migrations here as needed, incrementing version
    ];

    let mut status = String::new();
    let mut applied = 0;
    for (version, sql, desc) in migrations.iter() {
        if *version > current_version {
            match conn.execute(sql, []) {
                Ok(_) => {
                    let now = chrono::Utc::now().to_rfc3339();
                    conn.execute(
                        "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
                        params![version, now],
                    ).map_err(|e| format!("Failed to record schema version: {e}"))?;
                    writeln!(status, "Applied migration v{version}: {desc}").ok();
                    applied += 1;
                },
                Err(e) => {
                    writeln!(status, "Failed migration v{version}: {desc} -- {e}").ok();
                    return Err(status);
                }
            }
        } else {
            writeln!(status, "Skipped migration v{version}: {desc} (already applied)").ok();
        }
    }
    if applied == 0 {
        writeln!(status, "No new migrations needed. Schema is up to date.").ok();
    }
    Ok(status)
}

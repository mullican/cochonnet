use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub mod schema;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data directory");

        std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");

        let db_path: PathBuf = app_dir.join("petanque.db");
        let conn = Connection::open(&db_path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        schema::create_tables(&conn)?;
        Ok(())
    }
}

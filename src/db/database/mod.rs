use crate::db::database::traits::Create;
use crate::types::error::OnConflict;
use diesel::Connection;
use diesel::RunQueryDsl;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::cell::RefCell;
use std::sync::{LazyLock, Mutex};

pub mod authorize;
pub mod tables;
pub mod traits;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub const URL: &str = url();

/// Shared database handle — a single `SqliteConnection` protected by a
/// global mutex so that concurrent gRPC handlers never contend at the
/// SQLite file level (which would produce "database is locked" errors).
pub struct Db(Mutex<RefCell<diesel::SqliteConnection>>);

impl Db {
    fn init() -> Self {
        let conn =
            diesel::SqliteConnection::establish(URL).expect("Failed to open database");
        let conn = setup_conn(conn).expect("Failed to initialise database");
        Db(Mutex::new(RefCell::new(conn)))
    }

    /// Run a closure with a mutable reference to the underlying connection.
    /// Blocks if another thread is currently using the connection.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut diesel::SqliteConnection) -> R,
    {
        let guard = self.0.lock().unwrap();
        f(&mut *guard.borrow_mut())
    }
}

pub static CONN: LazyLock<Db> = LazyLock::new(Db::init);

fn setup_conn(
    mut conn: diesel::SqliteConnection,
) -> Result<diesel::SqliteConnection, Box<dyn std::error::Error + Send + Sync>> {
    conn.run_pending_migrations(MIGRATIONS)?;
    diesel::sql_query(PRAMGMAS).execute(&mut conn)?;
    let user = crate::types::user::User::invite_super(
        "0759762268",
        "Abdihakim Osman",
        Some("abdulhakimuthman100@gmail.com"),
    )?;
    conn.create(user).resolve()?;
    Ok(conn)
}

const PRAMGMAS: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 30000;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -65536;
PRAGMA mmap_size = 268435456;
"#;

const fn url() -> &'static str {
    match option_env!("DATABASE_URL") {
        Some(url) => url,
        None => "database.db",
    }
}

#[cfg(test)]
pub fn test_conn() -> diesel::SqliteConnection {
    diesel::SqliteConnection::establish(":memory:").expect("failed to open in-memory SQLite")
}

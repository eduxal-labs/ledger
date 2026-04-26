use crate::db::database::traits::Create;
use crate::types::error::OnConflict;
use diesel::RunQueryDsl;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub mod authorize;
pub mod tables;
pub mod traits;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type Database = std::cell::RefCell<diesel::SqliteConnection>;

pub const URL: &str = url();

thread_local! {
    pub static CONN: Database = <Database as New>::new(URL).unwrap();
}

pub trait New: Sized {
    fn new(url: &'static str) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>>;
}

impl New for diesel::SqliteConnection {
    fn new(url: &'static str) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        use diesel::Connection;
        let mut conn = Self::establish(url)?;
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
}

impl<T: New> New for std::cell::RefCell<T> {
    fn new(url: &'static str) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        Ok(Self::new(T::new(url)?))
    }
}

const PRAMGMAS: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
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
    use diesel::Connection;
    diesel::SqliteConnection::establish(":memory:").expect("failed to open in-memory SQLite")
}

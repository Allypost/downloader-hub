use std::time::Duration;

use app_config::CONFIG;
use app_migration::MigratorTrait;
use once_cell::sync::OnceCell;
use sea_orm::{Database, DatabaseConnection};

static APP_DB: OnceCell<AppDb> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct AppDb {
    pub conn: DatabaseConnection,
}

impl AppDb {
    pub async fn init() -> anyhow::Result<Self> {
        if let Some(db) = APP_DB.get() {
            return Ok(db.clone());
        }

        app_logger::debug!("Initializing database");

        let mut opt = sea_orm::ConnectOptions::new(&CONFIG.database.url);
        opt.max_connections(50)
            .connect_timeout(Duration::from_secs(5))
            .acquire_timeout(Duration::from_secs(5))
            .sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Trace);

        app_logger::debug!(opts = ?opt, "Connecting to database");

        let db = Database::connect(opt).await?;

        app_logger::info!("Connected to database");

        app_logger::trace!("Checking database connection");
        db.ping().await?;
        app_logger::trace!("Checked database connection");

        app_logger::info!("Running migrations");
        app_migration::Migrator::up(&db, None).await?;
        app_logger::info!("Migrations completed");

        let new = Self { conn: db };
        APP_DB.set(new).map_err(|e| {
            app_logger::error!(error = ?e, "Failed to set APP_DB");
            anyhow::anyhow!("Failed to set APP_DB: {:?}", e)
        })?;

        Ok(Self::global())
    }

    pub fn global() -> Self {
        APP_DB.get().expect("App database not initialized").clone()
    }

    pub fn db() -> DatabaseConnection {
        APP_DB
            .get()
            .expect("App database not initialized")
            .conn
            .clone()
    }
}

impl From<DatabaseConnection> for AppDb {
    fn from(db: DatabaseConnection) -> Self {
        Self { conn: db }
    }
}

impl From<AppDb> for DatabaseConnection {
    fn from(app_db: AppDb) -> Self {
        app_db.conn
    }
}

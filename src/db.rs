use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

const DATABASE_URL: &str = "sqlite:data/db.db?mode=rwc";

pub async fn init_db() -> DatabaseConnection {
    fs::create_dir_all("data")
        .await
        .expect("failed to create data directory");

    let db = Database::connect(DATABASE_URL)
        .await
        .expect("failed to connect database");

    Migrator::up(&db, None)
        .await
        .expect("failed to run database migration");

    db
}

use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

const DATABASE_URL: &str = "sqlite:db.db?mode=rwc";

pub async fn init_db() -> DatabaseConnection {
    let db = Database::connect(DATABASE_URL)
        .await
        .expect("failed to connect database");

    Migrator::up(&db, None)
        .await
        .expect("failed to run database migration");

    db
}

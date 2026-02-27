use sea_orm::{Database, DatabaseConnection, Schema, entity::prelude::*};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "guild_members")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    pub name: String,
    pub employee_id: String, // student ID or teacher ID
    pub identity: String,    // "local", "senior", "teacher", "guest"
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub async fn init_db() -> DatabaseConnection {
    let db = Database::connect("sqlite:db.db?mode=rwc")
        .await
        .expect("Connect to database failed");

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    let create_table_stmt = schema
        .create_table_from_entity(Entity)
        .if_not_exists()
        .to_owned();

    let _ = db.execute(builder.build(&create_table_stmt)).await;
    db
}

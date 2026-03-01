use chrono::{Duration, Utc};
use entity::model::announcement;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn crawler_prune_seen(db: &DatabaseConnection, ttl_seconds: u64) -> Result<(), String> {
    let cutoff = Utc::now() - Duration::seconds(ttl_seconds as i64);
    let result = announcement::Entity::delete_many()
        .filter(announcement::Column::CreatedAt.lt(cutoff))
        .exec(db)
        .await
        .map_err(|err| format!("failed to prune seen keys: {err}"))?;

    println!("[crawler] pruned {} seen keys", result.rows_affected);
    Ok(())
}

use chrono::Utc;
use entity::model::calendar_event_seen;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

const PROVIDER_NAME: &str = "google-calendar";

pub(crate) async fn is_event_seen(
    db: &DatabaseConnection,
    calendar_id: &str,
    event_id: &str,
    event_start: &str,
) -> Result<bool, String> {
    let found = calendar_event_seen::Entity::find()
        .filter(calendar_event_seen::Column::Provider.eq(PROVIDER_NAME))
        .filter(calendar_event_seen::Column::CalendarId.eq(calendar_id))
        .filter(calendar_event_seen::Column::EventId.eq(event_id))
        .filter(calendar_event_seen::Column::EventStart.eq(event_start))
        .one(db)
        .await
        .map_err(|err| format!("query calendar_event_seen failed: {err}"))?;

    Ok(found.is_some())
}

pub(crate) async fn mark_event_seen(
    db: &DatabaseConnection,
    calendar_id: &str,
    event_id: &str,
    event_start: &str,
) -> Result<(), String> {
    let record = calendar_event_seen::ActiveModel {
        id: Default::default(),
        provider: Set(PROVIDER_NAME.to_string()),
        calendar_id: Set(calendar_id.to_string()),
        event_id: Set(event_id.to_string()),
        event_start: Set(event_start.to_string()),
        created_at: Set(Utc::now()),
    };

    record
        .insert(db)
        .await
        .map_err(|err| format!("insert calendar_event_seen failed: {err}"))?;

    Ok(())
}

use crate::app_config::AppConfig;
use sea_orm::DatabaseConnection;
use serenity::prelude::{RwLock, TypeMapKey};
use std::sync::Arc;

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Arc<RwLock<AppConfig>>;
}

pub struct DbKey;

impl TypeMapKey for DbKey {
    type Value = DatabaseConnection;
}

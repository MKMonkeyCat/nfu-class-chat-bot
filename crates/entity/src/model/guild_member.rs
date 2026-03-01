use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum MemberIdentity {
    #[sea_orm(string_value = "local")]
    Local,
    #[sea_orm(string_value = "senior")]
    Senior,
    #[sea_orm(string_value = "teacher")]
    Teacher,
    #[sea_orm(string_value = "guest")]
    Guest,
}

impl MemberIdentity {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "local" => Self::Local,
            "senior" => Self::Senior,
            "teacher" => Self::Teacher,
            _ => Self::Guest,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "guild_member")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    pub name: String,
    pub employee_id: String,
    pub identity: MemberIdentity,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

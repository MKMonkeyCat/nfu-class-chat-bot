use sea_orm::FromJsonQueryResult;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "announcement")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub category: String,
    pub source_name: String,
    pub title: String,
    pub url: String,
    pub content: String,
    pub time: String,
    pub tags: TagList,

    #[sea_orm(indexed)]
    pub implementation_at: DateTimeUtc,

    #[sea_orm(auto_create_time)]
    pub created_at: DateTimeUtc,

    #[sea_orm(indexed, unique)]
    pub seen_key: String,

    pub simhash: i64,
    #[sea_orm(indexed)]
    pub chunk0: u16,
    #[sea_orm(indexed)]
    pub chunk1: u16,
    #[sea_orm(indexed)]
    pub chunk2: u16,
    #[sea_orm(indexed)]
    pub chunk3: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct TagList(pub Vec<String>);

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

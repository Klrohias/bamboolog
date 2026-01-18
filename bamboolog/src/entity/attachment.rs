use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "attachments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub mime: String,
    #[sea_orm(unique, indexed)]
    pub hash: String,
    pub storage_engine_id: i32,
    pub path: String,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::storage_engine::Entity",
        from = "Column::StorageEngineId",
        to = "super::storage_engine::Column::Id"
    )]
    StorageEngine,
}

impl Related<super::storage_engine::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StorageEngine.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

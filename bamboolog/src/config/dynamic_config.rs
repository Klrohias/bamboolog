pub mod config_entries {

    use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
        ExprTrait, IntoActiveModel, QueryFilter,
    };
    use serde::de::DeserializeOwned;

    use crate::entity::config_entry;

    pub struct ConfigEntry(&'static str, i32);

    pub const JWT_CONFIG: ConfigEntry = ConfigEntry("system", 1);
    pub const THEME_SERVICE_CONFIG: ConfigEntry = ConfigEntry("system", 2);
    pub const SITE_CONFIG: ConfigEntry = ConfigEntry("system", 3);

    impl ConfigEntry {
        pub async fn get<T>(&self, database: &DatabaseConnection) -> Result<Option<T>, ConfigError>
        where
            T: DeserializeOwned,
        {
            Ok(match self.get_string(database).await? {
                None => None,
                Some(v) => Some(serde_json::from_str(&v)?),
            })
        }

        pub async fn set_string(
            &self,
            database: &DatabaseConnection,
            value: String,
        ) -> Result<(), DbErr> {
            let exists = config_entry::Entity::find()
                .filter(
                    config_entry::Column::Component
                        .eq(self.0)
                        .and(config_entry::Column::ConfigId.eq(self.1)),
                )
                .one(database)
                .await?;

            if let Some(exists) = exists {
                let mut active = exists.into_active_model();
                active.value = ActiveValue::Set(value);
                active.update(database).await?;
            } else {
                config_entry::ActiveModel {
                    id: ActiveValue::NotSet,
                    value: ActiveValue::Set(value),
                    component: ActiveValue::Set(self.0.to_owned()),
                    config_id: ActiveValue::Set(self.1),
                }
                .insert(database)
                .await?;
            }
            Ok(())
        }

        pub async fn get_string(
            &self,
            database: &DatabaseConnection,
        ) -> Result<Option<String>, DbErr> {
            match config_entry::Entity::find()
                .filter(
                    config_entry::Column::Component
                        .eq(self.0)
                        .and(config_entry::Column::ConfigId.eq(self.1)),
                )
                .one(database)
                .await
            {
                Ok(Some(v)) => Ok(Some(v.value)),
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ConfigError {
        #[error(transparent)]
        JsonErr(#[from] serde_json::Error),
        #[error(transparent)]
        DbErr(#[from] DbErr),
    }
}

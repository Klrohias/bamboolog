use std::{borrow::Borrow, collections::HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ThemeManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub layout_mapping: HashMap<String, String>,
    #[serde(default)]
    pub config: Vec<ThemeConfigField>,
}

impl ThemeManifest {
    pub fn map_layout_file(&self, name: impl Borrow<str>) -> String {
        match self.layout_mapping.get(name.borrow()) {
            Some(value) => value.to_owned(),
            None => name.borrow().to_owned(),
        }
    }

    pub fn resolve_config(
        &self,
        values: &Map<String, Value>,
        reject_unknown: bool,
    ) -> Result<Map<String, Value>, ThemeConfigError> {
        self.validate_config_schema()?;

        if reject_unknown {
            for key in values.keys() {
                if !self.config.iter().any(|field| field.key == *key) {
                    return Err(ThemeConfigError::UnknownField(key.clone()));
                }
            }
        }

        let mut resolved = Map::new();
        for field in &self.config {
            let value = match values.get(&field.key) {
                Some(value) => Some(value.clone()),
                None => field.default.as_ref().map(toml_value_to_json).transpose()?,
            };
            match value {
                Some(value) => {
                    field.validate_value(&value)?;
                    resolved.insert(field.key.clone(), value);
                }
                None if field.required => {
                    return Err(ThemeConfigError::RequiredField(field.key.clone()));
                }
                None => {}
            }
        }

        Ok(resolved)
    }

    pub fn validate_config_schema(&self) -> Result<(), ThemeConfigError> {
        let mut keys = std::collections::HashSet::new();
        for field in &self.config {
            if field.key.trim().is_empty() || !keys.insert(&field.key) {
                return Err(ThemeConfigError::InvalidSchema(format!(
                    "configuration keys must be unique and non-empty: `{}`",
                    field.key
                )));
            }
            if field.kind == ThemeConfigFieldType::Select && field.options.is_empty() {
                return Err(ThemeConfigError::InvalidSchema(format!(
                    "select field `{}` has no options",
                    field.key
                )));
            }
            if field
                .min
                .is_some_and(|min| field.max.is_some_and(|max| min > max))
            {
                return Err(ThemeConfigError::InvalidSchema(format!(
                    "field `{}` has a minimum greater than its maximum",
                    field.key
                )));
            }
            if let Some(default) = &field.default {
                field.validate_value(&toml_value_to_json(default)?)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfigField {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: ThemeConfigFieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub options: Vec<ThemeConfigOption>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

impl ThemeConfigField {
    fn validate_value(&self, value: &Value) -> Result<(), ThemeConfigError> {
        let valid_type = match self.kind {
            ThemeConfigFieldType::String | ThemeConfigFieldType::Select => value.is_string(),
            ThemeConfigFieldType::Boolean => value.is_boolean(),
            ThemeConfigFieldType::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            ThemeConfigFieldType::Number => value.is_number(),
            ThemeConfigFieldType::Json => true,
        };
        if !valid_type {
            return Err(ThemeConfigError::InvalidValue {
                key: self.key.clone(),
                message: format!("must be a {}", self.kind),
            });
        }
        if self.kind == ThemeConfigFieldType::Select
            && !self
                .options
                .iter()
                .any(|option| value.as_str() == Some(option.value.as_str()))
        {
            return Err(ThemeConfigError::InvalidValue {
                key: self.key.clone(),
                message: "must be one of the declared options".to_string(),
            });
        }
        if let Some(number) = value.as_f64() {
            if self.min.is_some_and(|min| number < min) || self.max.is_some_and(|max| number > max)
            {
                return Err(ThemeConfigError::InvalidValue {
                    key: self.key.clone(),
                    message: "is outside the allowed range".to_string(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeConfigFieldType {
    String,
    Boolean,
    Integer,
    Number,
    Select,
    Json,
}

impl std::fmt::Display for ThemeConfigFieldType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Select => "select",
            Self::Json => "json",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfigOption {
    pub label: String,
    pub value: String,
}

fn toml_value_to_json(value: &toml::Value) -> Result<Value, ThemeConfigError> {
    serde_json::to_value(value).map_err(|error| ThemeConfigError::InvalidSchema(error.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeConfigError {
    #[error("Unknown configuration field `{0}`")]
    UnknownField(String),
    #[error("Configuration field `{0}` is required")]
    RequiredField(String),
    #[error("Invalid value for `{key}`: {message}")]
    InvalidValue { key: String, message: String },
    #[error("Invalid theme configuration schema: {0}")]
    InvalidSchema(String),
}

#[cfg(test)]
mod tests {
    use super::ThemeManifest;
    use serde_json::json;

    fn manifest() -> ThemeManifest {
        toml::from_str(
            r#"
                [[config]]
                key = "subtitle"
                label = "Subtitle"
                type = "string"
                default = "A journal"

                [[config]]
                key = "layout"
                label = "Layout"
                type = "select"
                default = "journal"
                options = [{ label = "Journal", value = "journal" }, { label = "Notes", value = "notes" }]

                [[config]]
                key = "sidebar_width"
                label = "Sidebar width"
                type = "integer"
                default = 10
                min = 1
                max = 50

                [[config]]
                key = "navigation"
                label = "Navigation"
                type = "json"
                default = [{ label = "Home", url = "/" }]
            "#,
        )
        .unwrap()
    }

    #[test]
    fn merges_defaults_and_validates_overrides() {
        let manifest = manifest();
        let config = manifest
            .resolve_config(
                &serde_json::from_value(
                    json!({ "subtitle": "Personal notes", "sidebar_width": 20 }),
                )
                .unwrap(),
                true,
            )
            .unwrap();

        assert_eq!(config["subtitle"], "Personal notes");
        assert_eq!(config["layout"], "journal");
        assert_eq!(config["sidebar_width"], 20);
        assert_eq!(config["navigation"][0]["url"], "/");
    }

    #[test]
    fn rejects_unknown_or_invalid_values() {
        let manifest = manifest();
        let unknown = serde_json::from_value(json!({ "other": true })).unwrap();
        let invalid = serde_json::from_value(json!({ "sidebar_width": 0 })).unwrap();

        assert!(manifest.resolve_config(&unknown, true).is_err());
        assert!(manifest.resolve_config(&invalid, true).is_err());
    }

    #[test]
    fn accepts_any_json_value_for_json_fields() {
        let manifest = manifest();
        let config = manifest
            .resolve_config(
                &serde_json::from_value(
                    json!({ "navigation": [{ "label": "About", "url": "/about" }] }),
                )
                .unwrap(),
                true,
            )
            .unwrap();

        assert_eq!(config["navigation"][0]["label"], "About");
    }
}

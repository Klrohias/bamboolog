use std::{borrow::Borrow, collections::HashMap};

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct ThemeDefinition {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub layout_mapping: HashMap<String, String>,
}

impl ThemeDefinition {
    pub fn map_layout_file(&self, name: impl Borrow<str>) -> String {
        match self.layout_mapping.get(name.borrow()) {
            Some(value) => value.to_owned(),
            None => name.borrow().to_owned(),
        }
    }
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::finders::Finder;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub key: Option<String>,
    pub description: Option<String>,
    pub finder: Finder,
    max_time: Option<u32>,
}

impl Extension {
    pub fn new(
        name: String,
        key: Option<String>,
        description: Option<String>,
        finder: Finder,
        max_time: Option<u32>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().hyphenated().to_string(),
            name,
            key,
            description,
            finder,
            max_time,
        }
    }
}

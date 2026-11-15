use super::super::models::{Item, MetadataCache};
use super::Find;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyFinder {}

impl Find for EmptyFinder {
    fn init(&mut self) {
        debug!("Initializing EmptyFinder");
    }
    fn update(&mut self) {
        debug!("Updating EmptyFinder");
    }
    fn search(&self, _query: &str, _meta: &MetadataCache) -> Vec<Item> {
        debug!("Searching EmptyFinder");
        Vec::new()
    }
}

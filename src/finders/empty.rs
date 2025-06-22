use super::super::models::Item;
use super::Find;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyFinder {}

impl Find for EmptyFinder {
    fn init(&mut self) {
        debug!("Initializing EmptyFinder");
    }
    fn update(&mut self) {
        debug!("Updating EmptyFinder");
    }
    fn search(&self, _query: &str) -> Vec<Item> {
        debug!("Searching EmptyFinder");
        return Vec::new();
    }
}

impl Default for EmptyFinder {
    fn default() -> Self {
        EmptyFinder {}
    }
}

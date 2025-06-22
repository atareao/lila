use super::super::{models::Item, utils::now};
use super::Find;
use applications::{AppInfo, common::AppInfoContext};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationFinder {
    cache: Vec<Item>,
    updated_at: u128,
}

impl ApplicationFinder {
    pub fn new() -> Self {
        debug!("Creating ApplicationFinder");
        ApplicationFinder {
            cache: Vec::new(),
            updated_at: 0,
        }
    }
    pub fn load() -> Vec<Item> {
        debug!("Loading ApplicationFinder");
        let mut ctx = AppInfoContext::new(vec![]);
        ctx.refresh_apps().unwrap();
        let items = ctx
            .get_all_apps()
            .iter()
            .map(|app| Item::App(app.clone()))
            .collect();
        debug!("Items: {:?}", items);
        items
    }
}

impl Find for ApplicationFinder {
    fn init(&mut self) {
        debug!("Initializing ApplicationFinder");
        self.update();
    }

    fn update(&mut self) {
        debug!("Updating ApplicationFinder");
        self.cache = Self::load();
        self.updated_at = now();
    }
    fn search(&self, _query: &str) -> Vec<Item> {
        debug!("Searching ApplicationFinder");
        self.cache.clone()
    }
}

use super::super::{models::Item, utils::now};
use super::Find;
use applications::{AppInfo, common::AppInfoContext};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
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

/// Filters and scores items by fuzzy matching their names against a query.
pub fn filter_items(items: &[Item], query: &str) -> Vec<Item> {
    let query = query.trim();
    if query.is_empty() {
        return items.to_vec();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, Item)> = items
        .iter()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item.name(), query)
                .map(|s| (s, item.clone()))
        })
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().map(|(_, item)| item).collect()
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
    fn search(&self, query: &str) -> Vec<Item> {
        debug!("Searching ApplicationFinder");
        filter_items(&self.cache, query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use applications::App;
    use std::path::PathBuf;

    fn app(name: &str) -> Item {
        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: PathBuf::from(format!("/tmp/{name}.desktop")),
        })
    }

    #[test]
    fn filter_empty_query_returns_all() {
        let items = vec![app("Firefox"), app("Gimp"), app("Terminal")];
        let out = filter_items(&items, "");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name(), "Firefox");
    }

    #[test]
    fn filter_partial_case_insensitive() {
        let items = vec![app("Firefox"), app("Gimp"), app("Terminal")];
        let binding = filter_items(&items, "fir");
        let names: Vec<&str> = binding.iter().map(|i| i.name()).collect();
        assert!(names.contains(&"Firefox"));
        assert!(!names.contains(&"Gimp"));
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let items = vec![app("Firefox"), app("Gimp")];
        assert!(filter_items(&items, "zzzzzz").is_empty());
    }
}

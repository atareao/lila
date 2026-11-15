use super::super::{
    models::{Item, MetadataCache},
    utils::now,
};
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

/// Filters and scores items by fuzzy matching the query against each item's
/// name, keywords and categories.
///
/// The score of an item is the **maximum** match among those scopes, so finding
/// a hit through a keyword is neither penalised nor rewarded against finding it
/// through the name. An item that matches in no scope is dropped. An empty
/// query keeps every item, unchanged.
///
/// `meta` memoises each `.desktop`'s keywords and categories, so scrolling
/// through queries does not re-read the files (see [`MetadataCache`]).
pub fn filter_items(items: &[Item], query: &str, meta: &MetadataCache) -> Vec<Item> {
    let query = query.trim();
    if query.is_empty() {
        return items.to_vec();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, Item)> = items
        .iter()
        .filter_map(|item| {
            best_fuzzy_match(item, query, meta, &matcher).map(|score| (score, item.clone()))
        })
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().map(|(_, item)| item).collect()
}

/// Best fuzzy score of `item` against `query`, or `None` if no scope matches.
///
/// Scopes are the item's name plus **each** of its keywords and **each** of its
/// categories, matched separately (never concatenated), and the result is their
/// maximum. Both [`filter_items`] and [`rank_items`] use it so the fuzzy term
/// cannot diverge between the direct and the ranked paths.
pub(super) fn best_fuzzy_match(
    item: &Item,
    query: &str,
    meta: &MetadataCache,
    matcher: &SkimMatcherV2,
) -> Option<i64> {
    let keywords = meta.keywords(item);
    let categories = meta.categories(item);
    std::iter::once(item.name())
        .chain(keywords.iter().map(String::as_str))
        .chain(categories.iter().map(String::as_str))
        .filter_map(|scope| matcher.fuzzy_match(scope, query))
        .max()
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
    fn search(&self, query: &str, meta: &MetadataCache) -> Vec<Item> {
        debug!("Searching ApplicationFinder");
        filter_items(&self.cache, query, meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MetadataCache;
    use applications::App;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn app(name: &str) -> Item {
        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: PathBuf::from(format!("/tmp/{name}.desktop")),
        })
    }

    /// `Item::App` con un `.desktop` real dentro de `dir`.
    ///
    /// `Exec` apunta a un binario existente porque GLib descarta los `.desktop`
    /// cuyo ejecutable no resuelve; con `None` la clave se omite del fichero.
    fn app_with_desktop(
        dir: &Path,
        file: &str,
        name: &str,
        keywords: Option<&str>,
        categories: Option<&str>,
    ) -> Item {
        let path = dir.join(file);
        let mut body =
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec=/usr/bin/env\n");
        if let Some(keywords) = keywords {
            body.push_str(&format!("Keywords={keywords}\n"));
        }
        if let Some(categories) = categories {
            body.push_str(&format!("Categories={categories}\n"));
        }
        fs::write(&path, body).expect("write");

        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: path,
        })
    }

    #[test]
    fn filter_empty_query_returns_all() {
        let items = vec![app("Firefox"), app("Gimp"), app("Terminal")];
        let out = filter_items(&items, "", &MetadataCache::default());
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name(), "Firefox");
    }

    #[test]
    fn filter_partial_case_insensitive() {
        let items = vec![app("Firefox"), app("Gimp"), app("Terminal")];
        let binding = filter_items(&items, "fir", &MetadataCache::default());
        let names: Vec<&str> = binding.iter().map(|i| i.name()).collect();
        assert!(names.contains(&"Firefox"));
        assert!(!names.contains(&"Gimp"));
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let items = vec![app("Firefox"), app("Gimp")];
        assert!(filter_items(&items, "zzzzzz", &MetadataCache::default()).is_empty());
    }

    /// [TDD-RED-A2-05] `filter_items` encuentra una app por una palabra clave
    /// que no aparece en su nombre
    #[test]
    fn filter_matches_keyword_absent_from_name() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let items = vec![app_with_desktop(
            tmp.path(),
            "editor.desktop",
            "TextEditor",
            Some("photos;image;"),
            None,
        )];

        let found = filter_items(&items, "photos", &MetadataCache::default());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name(), "TextEditor");
    }

    /// [TDD-RED-A2-06] `filter_items` encuentra una app por una categoría que no
    /// aparece en su nombre
    #[test]
    fn filter_matches_category_absent_from_name() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let items = vec![app_with_desktop(
            tmp.path(),
            "calc.desktop",
            "Numbers",
            None,
            Some("Utility;Spreadsheet;"),
        )];

        let found = filter_items(&items, "spreadsheet", &MetadataCache::default());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name(), "Numbers");
    }

    /// [TDD-RED-A2-07] El nombre sigue coincidiendo, metadatos aparte
    #[test]
    fn filter_still_matches_by_name() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let items = vec![
            app_with_desktop(
                tmp.path(),
                "a.desktop",
                "Firefox",
                Some("web;"),
                Some("Network;"),
            ),
            app_with_desktop(tmp.path(), "b.desktop", "Gimp", None, Some("Graphics;")),
        ];

        let found = filter_items(&items, "fir", &MetadataCache::default());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name(), "Firefox");
    }
}

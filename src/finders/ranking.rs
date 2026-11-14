// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//
//! Ranking combinado de resultados.
//!
//! `rank_items` puntúa cada ítem como una combinación lineal ponderada de la
//! coincidencia difusa con la consulta, la frecuencia de lanzamientos y la
//! recencia del último uso, tomando los pesos de `Config`. Cuando el historial
//! no está disponible o está desactivado, cae a un ranking puramente difuso.

use std::collections::HashMap;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::history::{HistoryRepository, Usage, frequency_score, recency_score};
use crate::models::{Config, Item};

use super::filter_items;

/// Uso nulo con el que se puntúa un ítem sin historial registrado.
const NO_USAGE: Usage = Usage {
    launches: 0,
    first_used: 0,
    last_used: 0,
};

/// Ordena `items` por relevancia para `query`.
///
/// La puntuación de cada ítem es
/// `fuzzy_weight * difuso + frequency_weight * frecuencia + recency_weight * recencia`.
/// Si `query` no está vacía, los ítems que no coinciden se descartan; si está
/// vacía, el término difuso es `0.0` para todos y ordenan frecuencia y recencia.
/// El resultado se recorta a `cfg.max_results` en todas las rutas.
///
/// Si `cfg.history_enabled` es `false` o no hay repositorio de historial, el
/// orden es el de [`filter_items`] (solo coincidencia difusa).
pub fn rank_items(
    items: &[Item],
    query: &str,
    history: Option<&dyn HistoryRepository>,
    cfg: &Config,
    now_millis: i64,
) -> Vec<Item> {
    /// Recorta `items` al máximo de resultados configurado.
    ///
    /// Centraliza el respeto de `max_results` para que las rutas de respaldo
    /// (historial desactivado, ausente o con error de carga) no lo ignoren.
    fn cap(mut items: Vec<Item>, max_results: usize) -> Vec<Item> {
        items.truncate(max_results);
        items
    }

    if !cfg.history_enabled {
        return cap(filter_items(items, query), cfg.max_results);
    }
    let Some(history) = history else {
        return cap(filter_items(items, query), cfg.max_results);
    };
    let usage_by_id: HashMap<String, Usage> = match history.load_all() {
        Ok(all) => all.into_iter().collect(),
        Err(err) => {
            tracing::warn!("No se pudo cargar el historial, se usa solo el difuso: {err}");
            return cap(filter_items(items, query), cfg.max_results);
        }
    };

    let query = query.trim();
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(f64, Item)> = items
        .iter()
        .filter_map(|item| {
            let fuzzy = if query.is_empty() {
                0.0
            } else {
                matcher.fuzzy_match(item.name(), query)? as f64
            };
            let usage = usage_by_id.get(&item.id()).unwrap_or(&NO_USAGE);
            let score = cfg.fuzzy_weight * fuzzy
                + cfg.frequency_weight * frequency_score(usage)
                + cfg.recency_weight * recency_score(usage, now_millis);
            Some((score, item.clone()))
        })
        .collect();

    scored.sort_by(|left, right| right.0.total_cmp(&left.0));
    cap(
        scored.into_iter().map(|(_, item)| item).collect(),
        cfg.max_results,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryRepository, Result as HistoryResult, Usage};
    use crate::models::Item;
    use applications::App;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Instante fijo para que los tests sean deterministas.
    const NOW: i64 = 1_700_000_000_000;
    /// Milisegundos que tiene un día.
    const DAY_MILLIS: i64 = 86_400_000;

    /// Aplicación con un `.desktop` inexistente: `Item::id()` cae al path literal,
    /// de modo que `dir` distingue dos ítems con el mismo nombre.
    fn app_at(dir: &str, name: &str) -> Item {
        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: PathBuf::from(format!("/nonexistent-lila/{dir}/{name}.desktop")),
        })
    }

    fn usage(launches: u64, last_used: i64) -> Usage {
        Usage {
            launches,
            first_used: last_used,
            last_used,
        }
    }

    /// Repositorio de historial en memoria para los tests.
    #[derive(Default)]
    struct MemoryHistory {
        entries: RefCell<HashMap<String, Usage>>,
    }

    impl MemoryHistory {
        fn with(entries: &[(&str, Usage)]) -> Self {
            let mut map = HashMap::new();
            for (id, value) in entries {
                map.insert((*id).to_string(), *value);
            }
            Self {
                entries: RefCell::new(map),
            }
        }
    }

    impl HistoryRepository for MemoryHistory {
        fn record(&self, item_id: &str, now_millis: i64) -> HistoryResult<()> {
            let mut entries = self.entries.borrow_mut();
            entries
                .entry(item_id.to_string())
                .and_modify(|value| {
                    value.launches += 1;
                    value.last_used = now_millis;
                })
                .or_insert(usage(1, now_millis));
            Ok(())
        }

        fn usage(&self, item_id: &str) -> HistoryResult<Option<Usage>> {
            Ok(self.entries.borrow().get(item_id).copied())
        }

        fn load_all(&self) -> HistoryResult<Vec<(String, Usage)>> {
            Ok(self
                .entries
                .borrow()
                .iter()
                .map(|(id, value)| (id.clone(), *value))
                .collect())
        }
    }

    fn names(items: &[Item]) -> Vec<&str> {
        items.iter().map(Item::name).collect()
    }

    /// [TDD-RED-B2-05] La frecuencia desempata dos coincidencias idénticas.
    #[test]
    fn frequency_breaks_ties_between_equally_matching_items() {
        let items = vec![app_at("a", "Firefox"), app_at("b", "Firefox")];
        let used = items[1].id();
        let history = MemoryHistory::with(&[(used.as_str(), usage(9, NOW))]);

        let ranked = rank_items(&items, "fire", Some(&history), &Config::default(), NOW);

        assert_eq!(ranked[0].id(), used);
    }

    /// [TDD-RED-B2-06] Con consulta vacía dominan los usados recientemente.
    #[test]
    fn empty_query_ranks_recently_used_first() {
        let items = vec![app_at("a", "Alpha"), app_at("b", "Beta")];
        let recent = items[1].id();
        let history = MemoryHistory::with(&[(recent.as_str(), usage(1, NOW))]);

        let ranked = rank_items(&items, "", Some(&history), &Config::default(), NOW);

        assert_eq!(ranked[0].id(), recent);
    }

    /// [TDD-RED-B2-07] Una coincidencia difusa fuerte adelanta a un uso leve.
    #[test]
    fn strong_fuzzy_match_outranks_slight_usage() {
        let items = vec![app_at("a", "Cheese"), app_at("b", "Watch")];
        let old_usage = NOW - 365 * DAY_MILLIS;
        let slight = items[1].id();
        let history = MemoryHistory::with(&[(slight.as_str(), usage(1, old_usage))]);

        let matcher = SkimMatcherV2::default();
        let strong = matcher
            .fuzzy_match("Cheese", "ch")
            .expect("Cheese coincide");
        let weak = matcher.fuzzy_match("Watch", "ch").expect("Watch coincide");
        assert!(strong > weak, "fixture: {strong} debería superar a {weak}");

        let ranked = rank_items(&items, "ch", Some(&history), &Config::default(), NOW);

        assert_eq!(ranked[0].name(), "Cheese");
    }

    /// [TDD-RED-B2-08] Se respeta el límite configurable de resultados.
    #[test]
    fn respects_max_results_limit() {
        let items: Vec<Item> = (0..5)
            .map(|index| app_at("m", &format!("App{index}")))
            .collect();
        let config = Config {
            max_results: 2,
            ..Config::default()
        };

        let ranked = rank_items(&items, "app", Some(&MemoryHistory::default()), &config, NOW);

        assert_eq!(ranked.len(), 2);
    }

    /// [TDD-RED-B2-09] Con el historial desactivado el orden es fuzzy puro.
    #[test]
    fn disabled_history_uses_pure_fuzzy_order() {
        let items = vec![app_at("a", "Firefox"), app_at("b", "Gimp")];
        let overused = items[1].id();
        let history = MemoryHistory::with(&[(overused.as_str(), usage(50, NOW))]);
        let config = Config {
            history_enabled: false,
            ..Config::default()
        };

        let ranked = rank_items(&items, "", Some(&history), &config, NOW);

        assert_eq!(names(&ranked), names(&filter_items(&items, "")));
        assert_eq!(names(&ranked), vec!["Firefox", "Gimp"]);
    }

    /// [TDD-RED-B3-01] Con el historial desactivado también se respeta el límite.
    #[test]
    fn disabled_history_respects_max_results_limit() {
        let items: Vec<Item> = (0..5)
            .map(|index| app_at("m", &format!("App{index}")))
            .collect();
        let config = Config {
            history_enabled: false,
            max_results: 2,
            ..Config::default()
        };

        let ranked = rank_items(&items, "app", Some(&MemoryHistory::default()), &config, NOW);

        assert_eq!(ranked.len(), 2);
    }

    /// [TDD-RED-B2-10] Sin repositorio de historial se usa el ranking difuso.
    #[test]
    fn missing_history_falls_back_to_fuzzy_ranking() {
        let items = vec![app_at("a", "Firefox"), app_at("b", "Gimp")];

        let ranked = rank_items(&items, "fir", None, &Config::default(), NOW);

        assert_eq!(names(&ranked), vec!["Firefox"]);
    }

    /// [TDD-RED-B2-11] Cambiar los pesos cambia el orden de los resultados.
    #[test]
    fn configurable_weights_change_the_order() {
        let items = vec![app_at("a", "Terminal"), app_at("b", "Terminal2")];
        let used = items[1].id();
        let history = MemoryHistory::with(&[(used.as_str(), usage(100, NOW))]);

        let fuzzy_only = Config {
            frequency_weight: 0.0,
            recency_weight: 0.0,
            ..Config::default()
        };
        let by_fuzzy = rank_items(&items, "term", Some(&history), &fuzzy_only, NOW);

        let usage_first = Config {
            fuzzy_weight: 0.0,
            ..Config::default()
        };
        let by_usage = rank_items(&items, "term", Some(&history), &usage_first, NOW);

        assert_eq!(by_fuzzy[0].name(), "Terminal");
        assert_eq!(by_usage[0].name(), "Terminal2");
    }
}

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
use crate::models::{Config, IdCache, Item};

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
/// `ids` memoiza el id canónico de cada ítem: se consulta una vez por ítem y
/// pulsación en lugar de canonicalizar la ruta cada vez (ver [`IdCache`]). El
/// historial completo se carga una sola vez por búsqueda, nunca por ítem.
///
/// Si `cfg.history_enabled` es `false` o no hay repositorio de historial, el
/// orden es el de [`filter_items`] (solo coincidencia difusa).
pub fn rank_items(
    items: &[Item],
    ids: &IdCache,
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
            let usage = usage_by_id.get(&ids.id(item)).unwrap_or(&NO_USAGE);
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
    use crate::history::{HistoryError, HistoryRepository, Result as HistoryResult, Usage};
    use crate::models::Item;
    use applications::App;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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

    /// Repositorio que delega en [`MemoryHistory`] y cuenta las cargas completas.
    #[derive(Default)]
    struct CountingHistory {
        inner: MemoryHistory,
        load_all_calls: Cell<usize>,
    }

    impl HistoryRepository for CountingHistory {
        fn record(&self, item_id: &str, now_millis: i64) -> HistoryResult<()> {
            self.inner.record(item_id, now_millis)
        }

        fn usage(&self, item_id: &str) -> HistoryResult<Option<Usage>> {
            self.inner.usage(item_id)
        }

        fn load_all(&self) -> HistoryResult<Vec<(String, Usage)>> {
            self.load_all_calls.set(self.load_all_calls.get() + 1);
            self.inner.load_all()
        }
    }

    /// Repositorio cuyo `load_all` siempre falla, para cubrir la rama de error.
    struct FailingHistory;

    impl HistoryRepository for FailingHistory {
        fn record(&self, _item_id: &str, _now_millis: i64) -> HistoryResult<()> {
            Ok(())
        }

        fn usage(&self, _item_id: &str) -> HistoryResult<Option<Usage>> {
            Ok(None)
        }

        fn load_all(&self) -> HistoryResult<Vec<(String, Usage)>> {
            Err(HistoryError::LockPoisoned)
        }
    }

    /// `App` cuyo `.desktop` es un fichero real dentro de `dir`.
    fn app_in(dir: &std::path::Path, file: &str, name: &str) -> Item {
        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: dir.join(file),
        })
    }

    /// [TDD-RED-B2-05] La frecuencia desempata dos coincidencias idénticas.
    #[test]
    fn frequency_breaks_ties_between_equally_matching_items() {
        let items = vec![app_at("a", "Firefox"), app_at("b", "Firefox")];
        let used = items[1].id();
        let history = MemoryHistory::with(&[(used.as_str(), usage(9, NOW))]);

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "fire",
            Some(&history),
            &Config::default(),
            NOW,
        );

        assert_eq!(ranked[0].id(), used);
    }

    /// [TDD-RED-B2-06] Con consulta vacía dominan los usados recientemente.
    #[test]
    fn empty_query_ranks_recently_used_first() {
        let items = vec![app_at("a", "Alpha"), app_at("b", "Beta")];
        let recent = items[1].id();
        let history = MemoryHistory::with(&[(recent.as_str(), usage(1, NOW))]);

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "",
            Some(&history),
            &Config::default(),
            NOW,
        );

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

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "ch",
            Some(&history),
            &Config::default(),
            NOW,
        );

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

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "app",
            Some(&MemoryHistory::default()),
            &config,
            NOW,
        );

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

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "",
            Some(&history),
            &config,
            NOW,
        );

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

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "app",
            Some(&MemoryHistory::default()),
            &config,
            NOW,
        );

        assert_eq!(ranked.len(), 2);
    }

    /// [TDD-RED-B2-10] Sin repositorio de historial se usa el ranking difuso.
    #[test]
    fn missing_history_falls_back_to_fuzzy_ranking() {
        let items = vec![app_at("a", "Firefox"), app_at("b", "Gimp")];

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "fir",
            None,
            &Config::default(),
            NOW,
        );

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
        let by_fuzzy = rank_items(
            &items,
            &IdCache::default(),
            "term",
            Some(&history),
            &fuzzy_only,
            NOW,
        );

        let usage_first = Config {
            fuzzy_weight: 0.0,
            ..Config::default()
        };
        let by_usage = rank_items(
            &items,
            &IdCache::default(),
            "term",
            Some(&history),
            &usage_first,
            NOW,
        );

        assert_eq!(by_fuzzy[0].name(), "Terminal");
        assert_eq!(by_usage[0].name(), "Terminal2");
    }

    /// [TDD-RED-CACHE-01] El historial se carga una sola vez por búsqueda,
    /// independientemente del número de ítems a puntuar.
    #[test]
    fn loads_history_once_per_search() {
        let items: Vec<Item> = (0..5)
            .map(|index| app_at("m", &format!("App{index}")))
            .collect();
        let history = CountingHistory::default();

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "app",
            Some(&history),
            &Config::default(),
            NOW,
        );

        assert_eq!(ranked.len(), 5);
        assert_eq!(history.load_all_calls.get(), 1);
    }

    /// [TDD-RED-CACHE-02] Con consulta vacía la frecuencia ordena usos igual de recientes.
    #[test]
    fn empty_query_ranks_the_most_launched_first() {
        let items = vec![app_at("a", "Alpha"), app_at("b", "Beta")];
        let often = items[1].id();
        let seldom = items[0].id();
        let history = MemoryHistory::with(&[
            (often.as_str(), usage(9, NOW)),
            (seldom.as_str(), usage(1, NOW)),
        ]);

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "",
            Some(&history),
            &Config::default(),
            NOW,
        );

        assert_eq!(ranked[0].id(), often);
    }

    /// [TDD-RED-CACHE-03] Con consulta vacía y misma frecuencia gana el uso
    /// más reciente: un retraso de `HALF_LIFE_DAYS` días vale media recencia.
    #[test]
    fn empty_query_prefers_the_most_recent_usage_at_equal_frequency() {
        let items = vec![app_at("a", "Alpha"), app_at("b", "Beta")];
        let fresh = items[0].id();
        let stale = items[1].id();
        let history = MemoryHistory::with(&[
            (fresh.as_str(), usage(1, NOW)),
            (stale.as_str(), usage(1, NOW - 14 * DAY_MILLIS)),
        ]);

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "",
            Some(&history),
            &Config::default(),
            NOW,
        );

        assert_eq!(ranked[0].id(), fresh);
    }

    /// [TDD-RED-CACHE-04] El id canónico se resuelve una vez por ruta y se
    /// reutiliza entre búsquedas: retirar el fichero no cambia la clave del
    /// historial en la siguiente pulsación.
    #[test]
    fn canonical_id_is_memoized_across_searches() {
        let dir = TempDir::new().expect("TempDir::new()");
        let plain = dir.path().join("plain.desktop");
        let target = dir.path().join("target.desktop");
        fs::write(&plain, "[Desktop Entry]\nName=Plain\n").expect("write");
        fs::write(&target, "[Desktop Entry]\nName=Linked\n").expect("write");
        // El enlace hace que el path literal y el canónico difieran.
        let link = dir.path().join("linked.desktop");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let items = vec![
            app_in(dir.path(), "plain.desktop", "Plain"),
            app_in(dir.path(), "linked.desktop", "Linked"),
        ];
        let linked_id = items[1].id();
        let history = MemoryHistory::with(&[(linked_id.as_str(), usage(9, NOW))]);
        let cache = IdCache::default();

        let before = rank_items(&items, &cache, "", Some(&history), &Config::default(), NOW);
        assert_eq!(names(&before), vec!["Linked", "Plain"]);

        // El destino desaparece y el enlace queda colgado: sin caché `id()` cae
        // al path literal, el historial ya no casaría y el orden se invertiría.
        fs::remove_file(&target).expect("remove");
        assert_ne!(
            items[1].id(),
            linked_id,
            "fixture: el id sin caché deja de ser canónico"
        );

        let after = rank_items(&items, &cache, "", Some(&history), &Config::default(), NOW);
        assert_eq!(names(&after), vec!["Linked", "Plain"]);
    }

    /// [TDD-RED-B2-12] Si `load_all` falla se cae al orden difuso y el límite
    /// de resultados se respeta también en esa rama de error.
    #[test]
    fn history_load_error_falls_back_to_fuzzy_ranking_and_caps_results() {
        let items: Vec<Item> = (0..5)
            .map(|index| app_at("m", &format!("App{index}")))
            .collect();
        let config = Config {
            max_results: 2,
            ..Config::default()
        };

        let ranked = rank_items(
            &items,
            &IdCache::default(),
            "app",
            Some(&FailingHistory),
            &config,
            NOW,
        );

        let fuzzy_order = filter_items(&items, "app");
        let expected: Vec<&str> = names(&fuzzy_order)
            .into_iter()
            .take(config.max_results)
            .collect();

        assert_eq!(names(&ranked), expected);
        assert_eq!(ranked.len(), config.max_results);
    }
}

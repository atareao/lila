// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//
//! Memorización del historial completo.
//!
//! El ranking consulta todo el historial en cada pulsación y esa consulta es la
//! parte cara del filtrado. [`CachedHistory`] decora un [`HistoryRepository`]
//! para resolver `load_all` una sola vez y reutilizar el resultado hasta que se
//! registre un lanzamiento nuevo.

use std::cell::RefCell;
use std::fmt;

use super::{HistoryRepository, Result, Usage};

/// Repositorio que memoriza `load_all()` e invalida la memorización en cada
/// `record()` correcto.
///
/// Envuelve a otro [`HistoryRepository`] y guarda el resultado de la última
/// carga completa: mientras nadie registre un lanzamiento, las siguientes
/// llamadas a `load_all` devuelven una copia de lo memorizado sin tocar el
/// repositorio interno (ni, por tanto, la base de datos).
///
/// La memorización vive lo que viva esta instancia y LiLa es una aplicación de
/// instancia única y el único escritor del historial, así que ningún otro
/// proceso puede dejar obsoleto el valor guardado.
pub struct CachedHistory {
    /// Repositorio decorado, consultado solo cuando no hay valor memorizado.
    inner: Box<dyn HistoryRepository>,
    /// Último `load_all()` correcto, o `None` si hay que volver a consultar.
    cache: RefCell<Option<Vec<(String, Usage)>>>,
}

// `Box<dyn HistoryRepository>` no es `Debug`, de ahí la implementación manual.
impl fmt::Debug for CachedHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedHistory")
            .field("memoized", &self.cache.borrow().is_some())
            .finish_non_exhaustive()
    }
}

impl CachedHistory {
    /// Crea el decorador sobre `inner`, con la memorización vacía.
    pub fn new(inner: Box<dyn HistoryRepository>) -> Self {
        Self {
            inner,
            cache: RefCell::new(None),
        }
    }
}

impl HistoryRepository for CachedHistory {
    /// Registra el lanzamiento y, solo si se pudo guardar, invalida la
    /// memorización.
    ///
    /// Si el registro falla el historial sigue igual y lo memorizado continúa
    /// siendo válido, así que se conserva hasta el siguiente registro correcto.
    fn record(&self, item_id: &str, now_millis: i64) -> Result<()> {
        self.inner.record(item_id, now_millis)?;
        *self.cache.borrow_mut() = None;
        Ok(())
    }

    /// Delega en el repositorio interno sin memorizar: consultar un ítem suelto
    /// no invalida ni rellena la carga completa.
    fn usage(&self, item_id: &str) -> Result<Option<Usage>> {
        self.inner.usage(item_id)
    }

    /// Devuelve el historial completo, memorizando el resultado.
    ///
    /// Si ya hay un valor memorizado se devuelve una copia y el repositorio
    /// interno no se consulta. Si no lo hay se delega en él y se memoriza
    /// **solo** el `Ok`: un error no se guarda, de modo que la siguiente llamada
    /// vuelve a intentarlo. El préstamo del `RefCell` se suelta antes de llamar
    /// al interno para que este no pueda encontrarlo prestado.
    fn load_all(&self) -> Result<Vec<(String, Usage)>> {
        let memoized = self.cache.borrow().clone();
        if let Some(all) = memoized {
            return Ok(all);
        }
        let all = self.inner.load_all()?;
        *self.cache.borrow_mut() = Some(all.clone());
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryError, Result as HistoryResult};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Instante fijo para que los tests sean deterministas.
    const T0: i64 = 1_700_000_000_000;

    /// Construye un `Usage` con `launches` lanzamientos en el instante `last_used`.
    fn usage(launches: u64, last_used: i64) -> Usage {
        Usage {
            launches,
            first_used: last_used,
            last_used,
        }
    }

    /// Contadores compartidos con el test.
    ///
    /// El decorador toma posesión del repositorio interno, de modo que las
    /// cuentas tienen que vivir fuera de él para poder observarlas después.
    #[derive(Debug, Default)]
    struct Counters {
        load_all: Cell<usize>,
        record: Cell<usize>,
        usage: Cell<usize>,
    }

    /// Repositorio de historial en memoria que cuenta las llamadas recibidas y
    /// puede fallar a voluntad en `load_all` o en `record`.
    struct CountingHistory {
        entries: RefCell<HashMap<String, Usage>>,
        counters: Rc<Counters>,
        fail_load_all: bool,
        fail_record: bool,
    }

    impl CountingHistory {
        /// Repositorio vacío que no falla.
        fn new(counters: Rc<Counters>) -> Self {
            Self {
                entries: RefCell::new(HashMap::new()),
                counters,
                fail_load_all: false,
                fail_record: false,
            }
        }

        /// Repositorio con las entradas indicadas que no falla.
        fn with(counters: Rc<Counters>, entries: &[(&str, Usage)]) -> Self {
            let store = Self::new(counters);
            for (id, value) in entries {
                store.entries.borrow_mut().insert((*id).to_string(), *value);
            }
            store
        }

        /// Repositorio cuyo `load_all` siempre falla.
        fn failing_load_all(counters: Rc<Counters>) -> Self {
            Self {
                fail_load_all: true,
                ..Self::new(counters)
            }
        }

        /// Repositorio cuyo `record` siempre falla.
        fn failing_record(counters: Rc<Counters>) -> Self {
            Self {
                fail_record: true,
                ..Self::new(counters)
            }
        }
    }

    impl HistoryRepository for CountingHistory {
        fn record(&self, item_id: &str, now_millis: i64) -> HistoryResult<()> {
            self.counters.record.set(self.counters.record.get() + 1);
            if self.fail_record {
                return Err(HistoryError::LockPoisoned);
            }
            self.entries
                .borrow_mut()
                .entry(item_id.to_string())
                .and_modify(|value| {
                    value.launches += 1;
                    value.last_used = now_millis;
                })
                .or_insert(usage(1, now_millis));
            Ok(())
        }

        fn usage(&self, item_id: &str) -> HistoryResult<Option<Usage>> {
            self.counters.usage.set(self.counters.usage.get() + 1);
            Ok(self.entries.borrow().get(item_id).copied())
        }

        fn load_all(&self) -> HistoryResult<Vec<(String, Usage)>> {
            self.counters.load_all.set(self.counters.load_all.get() + 1);
            if self.fail_load_all {
                return Err(HistoryError::LockPoisoned);
            }
            Ok(self
                .entries
                .borrow()
                .iter()
                .map(|(id, value)| (id.clone(), *value))
                .collect())
        }
    }

    /// [TDD-RED-CACHE-05] Tres `load_all` seguidos solo consultan el repositorio
    /// interno una vez y devuelven los mismos datos.
    #[test]
    fn load_all_is_memoized_across_calls() {
        let counters = Rc::new(Counters::default());
        let inner =
            CountingHistory::with(Rc::clone(&counters), &[("firefox.desktop", usage(3, T0))]);
        let history = CachedHistory::new(Box::new(inner));

        let first = history.load_all().expect("load_all falló");
        let second = history.load_all().expect("load_all falló");
        let third = history.load_all().expect("load_all falló");

        assert_eq!(counters.load_all.get(), 1);
        assert_eq!(first, second);
        assert_eq!(second, third);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, "firefox.desktop");
        assert_eq!(first[0].1.launches, 3);
    }

    /// [TDD-RED-CACHE-06] Un `record` correcto invalida la memorización: la
    /// siguiente carga vuelve a consultar y ya ve el lanzamiento registrado.
    #[test]
    fn record_invalidates_the_cache() {
        let counters = Rc::new(Counters::default());
        let history = CachedHistory::new(Box::new(CountingHistory::new(Rc::clone(&counters))));

        let before = history.load_all().expect("load_all falló");
        assert!(before.is_empty());

        history.record("firefox.desktop", T0).expect("record falló");
        let after = history.load_all().expect("load_all falló");

        assert_eq!(counters.load_all.get(), 2);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].0, "firefox.desktop");
        assert_eq!(after[0].1.launches, 1);
        assert_eq!(after[0].1.last_used, T0);
    }

    /// [TDD-RED-CACHE-07] Un `load_all` fallido no se memoriza: el siguiente
    /// intento vuelve a consultar el repositorio interno.
    #[test]
    fn failed_load_all_is_not_memoized() {
        let counters = Rc::new(Counters::default());
        let inner = CountingHistory::failing_load_all(Rc::clone(&counters));
        let history = CachedHistory::new(Box::new(inner));

        assert!(history.load_all().is_err());
        assert!(history.load_all().is_err());

        assert_eq!(counters.load_all.get(), 2);
    }

    /// [TDD-RED-CACHE-08] Un `record` fallido deja la caché intacta: la base de
    /// datos no cambió y la siguiente carga no consulta de nuevo.
    #[test]
    fn failed_record_keeps_the_cache() {
        let counters = Rc::new(Counters::default());
        let inner = CountingHistory::failing_record(Rc::clone(&counters));
        let history = CachedHistory::new(Box::new(inner));

        let first = history.load_all().expect("load_all falló");
        assert!(history.record("firefox.desktop", T0).is_err());
        let second = history.load_all().expect("load_all falló");

        assert_eq!(counters.load_all.get(), 1);
        assert_eq!(counters.record.get(), 1);
        assert_eq!(first, second);
    }

    /// [TDD-RED-CACHE-09] `usage` delega siempre en el interno y no interfiere
    /// con la memorización de `load_all`.
    #[test]
    fn usage_delegates_without_memoization() {
        let counters = Rc::new(Counters::default());
        let inner =
            CountingHistory::with(Rc::clone(&counters), &[("firefox.desktop", usage(2, T0))]);
        let history = CachedHistory::new(Box::new(inner));

        let loaded = history.load_all().expect("load_all falló");

        let first = history.usage("firefox.desktop").expect("usage falló");
        let second = history.usage("firefox.desktop").expect("usage falló");
        let unknown = history.usage("desconocido.desktop").expect("usage falló");

        assert_eq!(first, Some(usage(2, T0)));
        assert_eq!(second, first);
        assert_eq!(unknown, None);
        assert_eq!(counters.usage.get(), 3);

        let again = history.load_all().expect("load_all falló");
        assert_eq!(counters.load_all.get(), 1);
        assert_eq!(again, loaded);
    }
}

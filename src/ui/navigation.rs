// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

/// Número de filas que avanza o retrocede la selección con PageDown/PageUp.
pub const PAGE_STEP: usize = 10;

/// Movimiento de la selección dentro de la lista de resultados.
///
/// Las flechas envuelven en los extremos; el resto de movimientos quedan
/// acotados a los límites de la lista.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Flecha abajo: avanza una fila, envolviendo al principio.
    Down,
    /// Flecha arriba: retrocede una fila, envolviendo al final.
    Up,
    /// Avanza [`PAGE_STEP`] filas, acotando al final de la lista.
    PageDown,
    /// Retrocede [`PAGE_STEP`] filas, acotando al principio de la lista.
    PageUp,
    /// Primera fila.
    Home,
    /// Última fila.
    End,
}

/// Calcula el índice de la fila que debe quedar seleccionada.
///
/// `current` es la fila seleccionada actualmente (`None` si la lista está
/// vacía o no hay selección); `len` es el número de filas. Devuelve `None`
/// cuando la lista está vacía, de modo que el llamante no necesita
/// comprobar límites por su cuenta.
#[must_use]
pub fn target_index(current: Option<usize>, len: usize, mv: Move) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let last = len - 1;
    let target = match mv {
        Move::Down => match current {
            Some(c) => (c + 1) % len,
            None => 0,
        },
        Move::Up => match current {
            Some(c) => (c + last) % len,
            None => last,
        },
        Move::PageDown => match current {
            Some(c) => c.saturating_add(PAGE_STEP).min(last),
            None => PAGE_STEP.min(last),
        },
        Move::PageUp => match current {
            Some(c) => c.saturating_sub(PAGE_STEP).min(last),
            None => 0,
        },
        Move::Home => 0,
        Move::End => last,
    };
    Some(target)
}

/// Calcula el valor que debe tomar la `vadjustment` para que una fila quede
/// a la vista.
///
/// `row_top` y `row_bottom` son los bordes superior e inferior de la fila en
/// coordenadas del `listbox`; `current` es el valor actual de la `vadjustment`
/// y `page_size` su tamaño de página (la altura visible del `ScrolledWindow`).
/// Devuelve el nuevo valor de la `vadjustment`: `row_top` si la fila empieza
/// por encima del viewport o si es más alta que la propia página; si termina
/// por debajo, el valor que la alinea con el borde inferior; y `current` sin
/// cambios si ya está completamente visible.
#[must_use]
pub fn scroll_value(row_top: f64, row_bottom: f64, current: f64, page_size: f64) -> f64 {
    let row_height = row_bottom - row_top;
    if row_top < current || row_height > page_size {
        return row_top;
    }
    if row_bottom > current + page_size {
        return row_bottom - page_size;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_should_start_at_first_row_when_nothing_selected() {
        assert_eq!(target_index(None, 5, Move::Down), Some(0));
    }

    #[test]
    fn down_should_wrap_from_last_row_to_first() {
        assert_eq!(target_index(Some(4), 5, Move::Down), Some(0));
    }

    #[test]
    fn up_should_start_at_last_row_when_nothing_selected() {
        assert_eq!(target_index(None, 5, Move::Up), Some(4));
    }

    #[test]
    fn up_should_wrap_from_first_row_to_last() {
        assert_eq!(target_index(Some(0), 5, Move::Up), Some(4));
    }

    #[test]
    fn home_should_select_first_row() {
        assert_eq!(target_index(Some(7), 10, Move::Home), Some(0));
    }

    #[test]
    fn end_should_select_last_row() {
        assert_eq!(target_index(Some(1), 10, Move::End), Some(9));
    }

    #[test]
    fn page_down_should_advance_by_page_step() {
        assert_eq!(target_index(Some(2), 100, Move::PageDown), Some(12));
    }

    #[test]
    fn page_down_should_clamp_to_last_row_near_the_end() {
        assert_eq!(target_index(Some(95), 100, Move::PageDown), Some(99));
    }

    #[test]
    fn page_down_should_clamp_when_nothing_selected() {
        assert_eq!(target_index(None, 5, Move::PageDown), Some(4));
    }

    #[test]
    fn page_up_should_go_back_by_page_step() {
        assert_eq!(target_index(Some(30), 100, Move::PageUp), Some(20));
    }

    #[test]
    fn page_up_should_clamp_to_first_row_near_the_start() {
        assert_eq!(target_index(Some(3), 100, Move::PageUp), Some(0));
    }

    #[test]
    fn page_up_should_clamp_when_current_is_out_of_range() {
        assert_eq!(target_index(Some(50), 5, Move::PageUp), Some(4));
    }

    #[test]
    fn page_up_should_start_at_first_row_when_nothing_selected() {
        assert_eq!(target_index(None, 100, Move::PageUp), Some(0));
    }

    #[test]
    fn empty_list_should_never_yield_a_target() {
        for mv in [
            Move::Down,
            Move::Up,
            Move::PageDown,
            Move::PageUp,
            Move::Home,
            Move::End,
        ] {
            assert_eq!(target_index(None, 0, mv), None, "movimiento {mv:?}");
            assert_eq!(target_index(Some(0), 0, mv), None, "movimiento {mv:?}");
        }
    }

    #[test]
    fn single_row_list_should_always_stay_on_that_row() {
        assert_eq!(target_index(Some(0), 1, Move::Down), Some(0));
        assert_eq!(target_index(Some(0), 1, Move::Up), Some(0));
        assert_eq!(target_index(Some(0), 1, Move::PageDown), Some(0));
        assert_eq!(target_index(Some(0), 1, Move::PageUp), Some(0));
        assert_eq!(target_index(Some(0), 1, Move::Home), Some(0));
        assert_eq!(target_index(Some(0), 1, Move::End), Some(0));
    }

    #[test]
    fn equal_moves_should_be_compared_by_value() {
        assert_eq!(Move::Down, Move::Down);
        assert_ne!(Move::Down, Move::Up);
    }

    #[test]
    fn page_step_should_be_ten() {
        assert_eq!(PAGE_STEP, 10);
    }

    #[test]
    fn scroll_should_move_up_when_row_is_above_the_viewport() {
        assert_eq!(scroll_value(120.0, 150.0, 200.0, 300.0), 120.0);
    }

    #[test]
    fn scroll_should_move_down_when_row_is_below_the_viewport() {
        assert_eq!(scroll_value(400.0, 430.0, 0.0, 300.0), 130.0);
    }

    #[test]
    fn scroll_should_keep_current_value_when_row_is_already_visible() {
        assert_eq!(scroll_value(120.0, 150.0, 0.0, 300.0), 0.0);
    }

    #[test]
    fn scroll_should_align_top_when_row_is_taller_than_the_viewport() {
        assert_eq!(scroll_value(500.0, 900.0, 0.0, 300.0), 500.0);
    }
}

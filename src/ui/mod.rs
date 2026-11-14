// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

pub mod row;

/// Devuelve `true` si la lista de resultados debe mostrarse.
///
/// La lista es visible siempre que haya al menos un resultado, con
/// independencia de si la consulta está vacía (por ejemplo, al mostrar
/// el historial de uso).
#[must_use]
pub fn results_visible(count: usize) -> bool {
    count > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_visible_should_be_false_with_no_results() {
        assert!(!results_visible(0));
    }

    #[test]
    fn results_visible_should_be_true_with_one_result() {
        assert!(results_visible(1));
    }

    #[test]
    fn results_visible_should_be_true_with_several_results() {
        assert!(results_visible(5));
    }
}

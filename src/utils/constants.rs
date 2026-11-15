// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.

pub const APP_ID: &str = "es.atareao.lila";
pub const APP_QUALIFIER: &str = "es";
pub const APP_ORGANIZATION: &str = "atareao";
pub const APP_APPLICATION: &str = "lila";
pub const APP_NAME: &str = "lila";
pub const SCROLL_NAME: &str = "lila-scroll";
pub const LISTBOX_NAME: &str = "lila-listbox";
/// Acción que alterna la visibilidad del dashboard desde el bus de sesión.
///
/// `GApplication` exporta el action map por `org.gtk.Actions` en el object path
/// `/es/atareao/lila`, derivado de [`APP_ID`].
pub const TOGGLE_ACTION: &str = "toggle";
/// Acción que presenta el dashboard desde el bus de sesión (idempotente).
pub const PRESENT_ACTION: &str = "present";
/// Lado en píxeles del icono de cada fila de resultado.
pub const ICON_PIXEL_SIZE: i32 = 32;
/// Separación horizontal entre el icono y los textos de la fila.
pub const ICON_TEXT_SPACING: i32 = 10;

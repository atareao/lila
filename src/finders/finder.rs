// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.

use serde::{Deserialize, Serialize};

use super::super::models::Item;
use super::{ApplicationFinder, EmptyFinder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Finder {
    EmptyFinder(EmptyFinder),
    ApplicationFinder(ApplicationFinder),
}

impl Finder {
    pub fn init(&mut self) {
        match self {
            Finder::EmptyFinder(finder) => finder.init(),
            Finder::ApplicationFinder(finder) => finder.init(),
        }
    }
}

impl Default for Finder {
    fn default() -> Self {
        Finder::EmptyFinder(EmptyFinder::default())
    }
}

pub trait Find {
    fn init(&mut self);
    fn update(&mut self);
    fn search(&self, query: &str) -> Vec<Item>;
}

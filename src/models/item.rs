use applications::App;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    App(App),
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::App(app) => &app.name,
        }
    }
}

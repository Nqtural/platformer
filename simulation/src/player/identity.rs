use foundation::color::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    name: String,
    color: Color,
}

impl PlayerIdentity {
    pub fn new(name: String, color: Color) -> Self {
        Self { name, color }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn color(&self) -> &Color {
        &self.color
    }
}

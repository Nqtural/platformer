use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

#[derive(Deserialize, Serialize, Clone, SchemaRead, SchemaWrite, Debug)]
pub enum GameMode {
    Solos,
    Duos,
}

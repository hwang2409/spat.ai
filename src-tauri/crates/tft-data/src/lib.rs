use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Champion data from Data Dragon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionData {
    pub id: String,
    pub name: String,
    pub cost: u32,
    pub traits: Vec<String>,
}

/// Item data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    pub id: String,
    pub name: String,
    pub is_component: bool,
    pub recipe: Option<(String, String)>,
}

/// Meta composition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaComp {
    pub name: String,
    pub tier: String,
    pub core_units: Vec<String>,
    pub flex_units: Vec<String>,
    pub core_items: HashMap<String, Vec<String>>,
    pub early_game: Vec<String>,
    pub power_spike: String,
}

/// Game data registry (loaded from Data Dragon + meta files)
#[derive(Debug, Clone, Default)]
pub struct GameData {
    pub champions: HashMap<String, ChampionData>,
    pub items: HashMap<String, ItemData>,
    pub meta_comps: Vec<MetaComp>,
}

impl GameData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Placeholder - will load from data files in Phase 2+
    pub fn load() -> anyhow::Result<Self> {
        Ok(Self::new())
    }
}

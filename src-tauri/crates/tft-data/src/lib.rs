use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Champion data from Data Dragon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionData {
    pub id: String,
    pub name: String,
    pub cost: u32,
    pub traits: Vec<String>,
    pub icon: String,
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

/// Raw champions.json file format
#[derive(Debug, Deserialize)]
struct ChampionsFile {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    set: Option<u32>,
    champions: Vec<ChampionData>,
}

/// Game data registry
#[derive(Debug, Clone, Default)]
pub struct GameData {
    pub champions: HashMap<String, ChampionData>,
    pub champions_by_name: HashMap<String, String>,
    pub items: HashMap<String, ItemData>,
    pub meta_comps: Vec<MetaComp>,
}

impl GameData {
    /// Load champion data from the data directory
    pub fn load(data_dir: &Path) -> Result<Self> {
        let mut data = Self::default();

        let champions_path = data_dir.join("champions.json");
        if champions_path.exists() {
            let content = std::fs::read_to_string(&champions_path)
                .context("Failed to read champions.json")?;
            let file: ChampionsFile =
                serde_json::from_str(&content).context("Failed to parse champions.json")?;

            for champ in file.champions {
                let name_lower = champ.name.to_lowercase();
                data.champions_by_name
                    .insert(name_lower, champ.id.clone());
                data.champions.insert(champ.id.clone(), champ);
            }

            tracing::info!("Loaded {} champions", data.champions.len());
        } else {
            tracing::warn!(
                "No champions.json found at {}. Run scripts/fetch-templates.py",
                champions_path.display()
            );
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent() {
        let data = GameData::load(Path::new("/nonexistent")).unwrap();
        assert!(data.champions.is_empty());
    }
}

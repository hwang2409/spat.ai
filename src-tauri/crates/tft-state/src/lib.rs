use serde::{Deserialize, Serialize};

/// Represents the full game state extracted from screen capture
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameState {
    pub gold: u32,
    pub level: u32,
    pub stage: String,
    pub shop: Vec<ShopSlot>,
    pub bench: Vec<BoardSlot>,
    pub board: Vec<BoardSlot>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopSlot {
    pub index: usize,
    pub champion_id: Option<String>,
    pub champion_name: Option<String>,
    pub cost: Option<u32>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSlot {
    pub row: u32,
    pub col: u32,
    pub champion_id: Option<String>,
    pub star_level: u32,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub is_component: bool,
}

impl GameState {
    pub fn new() -> Self {
        Self::default()
    }
}

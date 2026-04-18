use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub name: String,
    pub package_id: String,   // Уникальный ID: "author.modname"
    pub version: String,
    pub author: String,
    pub supported_versions: Vec<String>,
    pub path: std::path::PathBuf,
    pub source: ModSource,

    // Зависимости и порядок загрузки
    pub dependencies: Vec<String>,      // packageId зависимостей
    pub load_after: Vec<String>,
    pub load_before: Vec<String>,
    pub incompatible_with: Vec<String>,

    // Состояние в менеджере
    pub is_active: bool,

    // Метаданные превью
    pub description: String,
    pub preview_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModSource {
    Core,           // Data/Core
    DLC(String),    // Data/Royalty, etc.
    Local,          // Mods/
    Workshop(u64),  // Steam Workshop ID
}
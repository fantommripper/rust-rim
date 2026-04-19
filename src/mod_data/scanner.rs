use std::path::Path;
use crate::mod_data::{ModEntry, ModSource};
use super::parser::parse_about_xml;

fn lower_ids(ids: Vec<String>) -> Vec<String> {
    ids.into_iter().map(|s| s.to_lowercase()).collect()
}

/// Ищет превью-изображение в папке `mod_dir/About/` без учёта регистра.
/// Поддерживает: Preview.png, preview.png, Preview.jpg, preview.jpeg и т.д.
fn find_preview_image(mod_dir: &Path) -> Option<std::path::PathBuf> {
    let about_dir = mod_dir.join("About");
    let entries = std::fs::read_dir(&about_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if matches!(lower.as_str(),
                "preview.png" | "preview.jpg" | "preview.jpeg" | "preview.webp" | "preview.gif"
            ) {
                return Some(path);
            }
        }
    }
    None
}

/// Сканирует папку `game_path/Data/` и возвращает Core + DLC как ModEntry.
/// Core всегда идёт первым, остальные — по алфавиту.
pub fn scan_dlc_mods(game_path: &Path) -> Vec<ModEntry> {
    let data_dir = game_path.join("Data");
    let mut result = Vec::new();

    let entries = match std::fs::read_dir(&data_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Cannot read Data directory {:?}: {}", data_dir, e);
            return result;
        }
    };

    let mut folders: Vec<_> = entries.flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    // Core первый, затем остальные по алфавиту
    folders.sort_by(|a, b| {
        let a_core = a.file_name() == "Core";
        let b_core = b.file_name() == "Core";
        match (a_core, b_core) {
            (true, false)  => std::cmp::Ordering::Less,
            (false, true)  => std::cmp::Ordering::Greater,
            _              => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in folders {
        let mod_dir = entry.path();
        let about_xml = mod_dir.join("About").join("About.xml");
        if !about_xml.exists() { continue; }

        let folder_name = mod_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let source = if folder_name == "Core" {
            ModSource::Core
        } else {
            ModSource::DLC(folder_name.clone())
        };

        match parse_about_xml(&about_xml) {
            Ok(data) => {
                result.push(ModEntry {
                    name: if data.name.is_empty() { folder_name } else { data.name },
                    package_id: data.package_id.to_lowercase(),
                    version: data.version,
                    author: data.author,
                    supported_versions: data.supported_versions,
                    path: mod_dir.clone(),
                    source,
                    dependencies:     lower_ids(data.dependencies),
                    load_after:       lower_ids(data.load_after),
                    load_before:      lower_ids(data.load_before),
                    incompatible_with: lower_ids(data.incompatible_with),
                    is_active: false,
                    description: data.description,
                    preview_path: find_preview_image(&mod_dir),
                });
            }
            Err(e) => {
                tracing::warn!("Skipping DLC {:?}: {}", about_xml, e);
            }
        }
    }

    result
}

pub fn scan_local_mods(mods_dir: &Path) -> Vec<ModEntry> {
    let mut result = Vec::new();

    let entries = match std::fs::read_dir(mods_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Cannot read mods directory {:?}: {}", mods_dir, e);
            return result;
        }
    };

    for entry in entries.flatten() {
        let mod_dir = entry.path();
        if !mod_dir.is_dir() {
            continue;
        }

        let about_xml = mod_dir.join("About").join("About.xml");
        if !about_xml.exists() {
            continue;
        }

        match parse_about_xml(&about_xml) {
            Ok(data) => {
                let folder_name = mod_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let source = if folder_name.chars().all(|c| c.is_ascii_digit()) {
                    folder_name
                        .parse::<u64>()
                        .map(ModSource::Workshop)
                        .unwrap_or(ModSource::Local)
                } else {
                    ModSource::Local
                };

                result.push(ModEntry {
                    name: if data.name.is_empty() { folder_name } else { data.name },
                    package_id: data.package_id.to_lowercase(),
                    version: data.version,
                    author: data.author,
                    supported_versions: data.supported_versions,
                    path: mod_dir.clone(),
                    source,
                    dependencies:     lower_ids(data.dependencies),
                    load_after:       lower_ids(data.load_after),
                    load_before:      lower_ids(data.load_before),
                    incompatible_with: lower_ids(data.incompatible_with),
                    is_active: false,
                    description: data.description,
                    preview_path: find_preview_image(&mod_dir),
                });
            }
            Err(e) => {
                tracing::warn!("Skipping {:?}: {}", about_xml, e);
            }
        }
    }

    result
}

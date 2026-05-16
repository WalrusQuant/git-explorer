use super::helpers::config_path;
use super::types::Config;
use std::path::Path;

#[tauri::command]
pub fn load_config() -> Result<Config, String> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config {
            root_path: String::new(),
        });
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    Ok(config)
}

#[tauri::command]
pub fn save_config(root_path: String) -> Result<(), String> {
    let path = config_path()?;

    let validated = Path::new(&root_path);
    if !validated.exists() || !validated.is_dir() {
        return Err(format!("Invalid directory: {}", root_path));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config = Config { root_path };
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::{env, fs};

pub fn find_resource_path(filename: &str) -> Result<PathBuf> {
    let mut tried_paths = Vec::new();

    // project root
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get project root")
        .join(filename);
    tried_paths.push(local.clone());
    if local.exists() {
        return Ok(local);
    }

    // user data directory (usually ~/.local/share/platformer)
    if let Some(data_dir) = dirs::data_dir() {
        let user_path = data_dir.join("platformer").join(filename);
        tried_paths.push(user_path.clone());
        if user_path.exists() {
            return Ok(user_path);
        }
    }

    // construct a readable error listing all attempted paths
    let paths_str = tried_paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("'\n'");

    Err(anyhow!("Resource '{}' not found. Searched paths:\n'{}'", filename, paths_str))
}

pub fn load_resource_bytes(filename: &str) -> Result<Vec<u8>> {
    let path = find_resource_path(filename)?;
    fs::read(&path)
        .map_err(|e| anyhow!("Failed to read '{}': {}", path.display(), e))
}

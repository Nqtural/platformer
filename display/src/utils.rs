use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::{env, fs};
use ggez::graphics::{
    Color as GgezColor,
    Rect as GgezRect,
};
use foundation::color::Color;
use foundation::rect::Rect;
use glam::Vec2;
use mint::{Point2, Vector2};

pub trait IntoMint {
    fn to_mint_point(self) -> Point2<f32>;
    fn to_mint_vec(self) -> Vector2<f32>;
}

impl IntoMint for Vec2 {
    fn to_mint_point(self) -> Point2<f32> {
        Point2 { x: self.x, y: self.y }
    }

    fn to_mint_vec(self) -> Vector2<f32> {
        Vector2 { x: self.x, y: self.y }
    }
}

pub fn color_to_ggez(color: &Color) -> GgezColor {
    GgezColor::new(
        color.r,
        color.g,
        color.b,
        color.a,
    )
}

pub fn rect_to_ggez(rect: &Rect) -> GgezRect {
    GgezRect::new(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    )
}

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
        .join("' or '");

    Err(anyhow!("Resource '{}' not found in '{}'", filename, paths_str))
}

pub fn load_resource_bytes(filename: &str) -> Result<Vec<u8>> {
    let path = find_resource_path(filename)?;
    fs::read(&path)
        .map_err(|e| anyhow!("Failed to read '{}': {}", path.display(), e))
}

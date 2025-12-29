use anyhow::Result;
use serde::Deserialize;
use toml;
use foundation::color::Color;
use crate::utils::{
    find_resource_path,
    load_resource_bytes,
};

#[derive(Deserialize)]
pub struct Config {
    player: Player,
    teams: Teams,
    camera: Camera,
    client: ClientConfig,
    server: ServerConfig,
}

#[derive(Deserialize)]
struct Player {
    name: String,
}

#[derive(Deserialize)]
struct Teams {
    team_one_color: Color,
    team_two_color: Color,
}

#[derive(Deserialize)]
struct Camera {
    bias: f32,
    zoom: f32,
    vsync: bool,
    player_name_above: bool,
}

#[derive(Deserialize)]
struct ClientConfig {
    ip: String,
    port: String,
}

#[derive(Deserialize)]
struct ServerConfig {
    ip: String,
    port: String,
    team_size: usize,
}

impl Config {
    pub fn get() -> Result<Self> {
        let toml_str = std::fs::read_to_string(find_resource_path("config.toml")?)?;
        let config: Config = toml::from_str(&toml_str)?;
        Ok(config)
    }

    #[must_use]
    pub fn playername(&self) -> &str { &self.player.name }

    #[must_use]
    pub fn team_one_color(&self) -> Color {
        self.teams.team_one_color.clone()
    }

    #[must_use]
    pub fn team_two_color(&self) -> Color {
        self.teams.team_two_color.clone()
    }

    pub fn background_image(&self) -> Result<Vec<u8>> {
        load_resource_bytes("assets/background.png")
    }

    pub fn attack_image(&self) -> Result<Vec<u8>> {
        load_resource_bytes("assets/normal.png")
    }

    pub fn parry_image(&self) -> Result<Vec<u8>> {
        load_resource_bytes("assets/parry.png")
    }

    #[must_use]
    pub fn serverip(&self) -> &str { &self.server.ip }

    #[must_use]
    pub fn serverport(&self) -> &str { &self.server.port }

    #[must_use]
    pub fn team_size(&self) -> usize { self.server.team_size }

    #[must_use]
    pub fn clientip(&self) -> &str { &self.client.ip }

    #[must_use]
    pub fn clientport(&self) -> &str { &self.client.port }

    #[must_use]
    pub fn camera_bias(&self) -> f32 { self.camera.bias }

    #[must_use]
    pub fn camera_zoom(&self) -> f32 { self.camera.zoom }

    #[must_use]
    pub fn vsync(&self) -> bool { self.camera.vsync }

    #[must_use]
    pub fn player_name_above(&self) -> bool { self.camera.player_name_above }
}

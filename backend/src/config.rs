use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ServersConfig {
    pub servers: Vec<ServerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: i32,
    pub name: String,
    pub description: String,
}

impl ServersConfig {
    pub fn from_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }
}

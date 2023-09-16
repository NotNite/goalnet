use std::path::{Path, PathBuf};
use anyhow::Context;
use serde::Deserialize;

// https://github.com/serde-rs/serde/issues/1030
fn default_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub goalnet: ConfigGoalnet,
    pub process: ConfigProcess,
    pub payload: ConfigPayload,
    pub entrypoint: ConfigEntrypoint,
}

#[derive(Deserialize, Debug)]
pub struct ConfigGoalnet {
    pub path: Option<String>,

    #[serde(default = "default_false")]
    pub stderr_log: bool,

    #[serde(default = "default_false")]
    pub msgbox_log: bool,
}

#[derive(Deserialize, Debug)]
pub struct ConfigProcess {
    pub name: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct ConfigPayload {
    pub directory: PathBuf,
    pub dll: String,
    pub runtime_config: String,

    #[serde(default = "default_false")]
    pub copy_build_artifacts: bool,
}

#[derive(Deserialize, Debug)]
pub struct ConfigEntrypoint {
    pub type_name: String,
    pub method_name: String,
    pub delegate_type_name: String,

    #[serde(default = "default_false")]
    pub unload: bool,
}

pub fn parse(path: &Path) -> anyhow::Result<Config> {
    let config = toml::from_str(&std::fs::read_to_string(path)?)?;
    Ok(config)
}

pub fn temp_filename() -> String {
    let unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_millis();

    format!("goalnet-{}", unix_ms.to_string())
}

pub fn relative_dir(config_path: &Path, dir: &Path) -> anyhow::Result<PathBuf> {
    if dir.is_relative() {
        let config_path = config_path.canonicalize()
            .context("failed to canonicalize config file path")?;
        let mut config_dir = config_path;
        config_dir.pop();
        config_dir.push(dir);
        Ok(config_dir)
    } else {
        dir.canonicalize()
            .context("failed to canonicalize path")
    }
}
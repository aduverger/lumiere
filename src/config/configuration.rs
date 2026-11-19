use crate::error::LumiereError;
use dirs::home_dir;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use crate::Cli;

/// Settings read from `~/.config/lumiere/lumiere.config.json`, or from the file
/// given with `--config`. Command-line flags take precedence.
#[derive(Debug, Deserialize, Default)]
pub struct LumiereConfig {
    #[serde(default)]
    pub theme: Option<String>,

    #[serde(default)]
    pub wrap: Option<bool>,
}

fn default_config_path() -> Option<PathBuf> {
    let path = home_dir()?.join(".config/lumiere/lumiere.config.json");
    path.exists().then_some(path)
}

impl LumiereConfig {
    pub fn build(cli: &Cli) -> Result<Self, LumiereError> {
        match cli.config.as_ref().map(PathBuf::from).or_else(default_config_path) {
            Some(path) => {
                let reader = BufReader::new(File::open(path)?);
                serde_json::from_reader(reader)
                    .map_err(|error| LumiereError::InvalidConfiguration(error.to_string()))
            }
            None => Ok(LumiereConfig::default()),
        }
    }
}

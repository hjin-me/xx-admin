use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct BaseConf {
    pub white_list: Option<Vec<i64>>,
    pub black_list: Option<Vec<i64>>,
}

impl BaseConf {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let conf = toml::from_str::<Self>(&content)?;
        Ok(conf)
    }
}

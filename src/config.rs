use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub org_id: u64,
    pub xx_org_gray_id: String,
    pub admin_user: String,
    pub wechat_proxy: String,
    pub notice_bot: Vec<String>,
    pub proxy_server: Option<String>,
    pub exec_hour: u32,
    pub exec_minute: u32,
}

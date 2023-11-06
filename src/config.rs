use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub org_id: u64,                  // orgId
    pub xx_org_gray_id: String,       // orgGrayId
    pub admin_user: String,           // 学习管理员的企业微信ID
    pub wechat_proxy: String,         // 一个代理 https://github.com/hjin-me/wechat-proxy
    pub notice_bot: Vec<String>,      // 企业微信群机器人 URL
    pub proxy_server: Option<String>, // 代理服务器地址
    pub exec_hour: u32,               // 发送通知的小时
    pub exec_minute: u32,             // 发送通知的分钟
}

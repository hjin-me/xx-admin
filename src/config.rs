use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdminConfig {
    pub org_id: u64,                  // orgId
    pub xx_org_gray_id: String,       // orgGrayId
    pub admin_user: String,           // 学习管理员的企业微信ID
    pub notice_bot: Vec<String>,      // 企业微信群机器人 URL
    pub proxy_server: Option<String>, // 代理服务器地址
    pub exec_hour: u32,               // 发送通知的小时
    pub exec_minute: u32,             // 发送通知的分钟

    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudyConfig {
    pub proxy_server: Option<String>,  // 代理服务器地址
    pub study_schedule: Vec<Schedule>, // 学习计划
    pub app_caller: String,

    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schedule {
    pub target: String,
    pub hour: u32,   //发送通知的小时
    pub minute: u32, //发送通知的分钟
}

use anyhow::Error;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Member {
    #[serde(rename = "rangeRealScore")]
    pub range_real_score: u64,
    #[serde(rename = "deptNames")]
    pub dept_names: String,
    #[serde(rename = "scoreMonth")]
    pub score_month: u64,
    #[serde(rename = "rangeScore")]
    pub range_score: u64,
    #[serde(rename = "deptIds")]
    pub dept_ids: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userId")]
    pub user_id: u64,
    #[serde(rename = "totalScore")]
    pub total_score: u64,
    #[serde(rename = "orgId")]
    pub org_id: i64,
    #[serde(rename = "isActivate")]
    pub is_activate: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MemberScore {
    pub date: String,
    pub count: i64,
    pub data: Vec<Member>,
    pub organization_rank: Vec<OrganizationRank>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrganizationRank {
    pub rank: u64,
    #[serde(rename = "orgName")]
    pub org_name: String,
    #[serde(rename = "orgId")]
    pub org_id: u64,
    #[serde(rename = "statDate")]
    pub stat_date: String,
    #[serde(rename = "avgScore")]
    pub avg_score: f32,
    #[serde(rename = "preDiffScore")]
    pub pre_diff_score: f32,
}
pub enum StateChange {
    BrowserClosed(Error),
    Init,
    Ready,
    WaitingLogin(String),
    LoggedIn,
    Complete(MemberScore),
}
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Ticket {
    pub ticket: String,
}
#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum State {
    Broken(String),
    Prepare,
    Init,
    Ready,
    WaitingLogin((String, i64)),
    Logged,
    Complete(MemberScore),
}

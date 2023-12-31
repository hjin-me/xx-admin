#[cfg(feature = "server")]
use anyhow::Error;
use serde::{Deserialize, Serialize};
#[cfg(feature = "hydrate")]
#[derive(Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub uid: i64,
    pub nick: String,
    // avatarMediaUrl: String,
}

#[cfg(feature = "server")]
pub enum StateChange {
    BrowserClosed(Error),
    Init,
    Ready,
    WaitingLogin(String),
    LoggedIn(UserInfo),
    StartLearn,
    LearnLog((String, Vec<(String, i64, i64)>)),
    Complete((String, i64)),
}
#[cfg(feature = "hydrate")]
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Ticket {
    pub ticket: String,
}
#[cfg(feature = "hydrate")]
#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum State {
    Broken(String),
    Prepare,
    Init,
    Ready,
    WaitingLogin((String, i64)),
    Logged(String),
    Learning((String, Vec<(String, i64, i64)>)),
    Complete((String, i64)),
}

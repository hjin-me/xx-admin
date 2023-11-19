#[cfg(feature = "server")]
use anyhow::Error;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
pub enum StateChange {
    BrowserClosed(Error),
    Init,
    Ready,
    WaitingLogin(String),
    LoggedIn(String),
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

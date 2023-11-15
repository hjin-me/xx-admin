mod msg;

use anyhow::{anyhow, Result};
use chrono::{Duration, Local};
pub use msg::*;
use reqwest::Client;
use serde::Deserialize;
use std::ops::Add;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::trace;

struct Token {
    content: String,
    expires_after: chrono::DateTime<Local>,
}
#[derive(Debug, Deserialize, Clone)]
struct AccessTokenResp {
    errcode: isize,                   // `json:"errcode"`
    errmsg: String,                   // `json:"errmsg"`
    pub access_token: Option<String>, // `json:"access_token" validate:"required"`
    pub expires_in: Option<i64>,      // `json:"expires_in" validate:"required"`
}
#[derive(Clone)]
pub struct MP {
    corp_id: String,
    corp_secret: String,
    agent_id: i64,
    access_token: Arc<RwLock<Token>>,
    client: Client,
}

impl MP {
    pub fn new(corp_id: &str, corp_secret: &str, agent_id: i64) -> Self {
        Self {
            corp_id: corp_id.to_string(),
            corp_secret: corp_secret.to_string(),
            agent_id,
            access_token: Arc::new(RwLock::new(Token {
                content: "".to_string(),
                expires_after: Local::now(),
            })),
            client: Client::new(),
        }
    }
    async fn get_access_token(&self) -> Result<(String, i64)> {
        let r = self
            .client
            .get(format!(
                "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
                self.corp_id, self.corp_secret
            ))
            .send()
            .await?
            .json::<AccessTokenResp>()
            .await?;
        if r.errcode != 0 {
            return Err(anyhow!("errcode: {}, errmsg: {}", r.errcode, r.errmsg));
        }
        if let (Some(access_token), Some(expires_in)) = (r.access_token, r.expires_in) {
            return Ok((access_token, expires_in));
        }
        Err(anyhow!("access_token or expires_in is None"))
    }
    async fn refresh_token(&self) -> Result<()> {
        trace!("refresh_token");
        let (access_token, expires_in) = self.get_access_token().await?;
        let mut w = self.access_token.write().await;
        w.content = access_token;
        w.expires_after = Local::now().add(Duration::seconds(expires_in - 30));
        Ok(())
    }

    async fn get_token(&self) -> Result<String> {
        let token = self.access_token.read().await;
        if token.expires_after < Local::now() {
            drop(token);
            self.refresh_token().await?;
        }
        let r = self.access_token.read().await;
        Ok(r.content.clone())
    }
}

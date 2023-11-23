use crate::backend::xxscore::{daily_score, XxAdmin};
use crate::state::State;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use study_core::utils::UserValidator;
use wx::MP;

#[derive(Clone)]
pub struct StateSession {
    data: Arc<RwLock<XxAdmin>>,
    mp: MP,
    xx_org_gray_id: String,
    proxy_server: Option<String>,

    wechat_bots: Vec<String>,
    org_id: u64,
    admin_user: String,
}

impl StateSession {
    pub fn new(
        mp: MP,
        xx_org_gray_id: &str,
        proxy_server: Option<String>,
        wechat_bots: Vec<String>,
        org_id: u64,
        admin_user: String,
    ) -> Result<Self> {
        Ok(Self {
            data: Arc::new(RwLock::new(XxAdmin::new(
                xx_org_gray_id,
                proxy_server.clone(),
            )?)),
            mp,
            xx_org_gray_id: xx_org_gray_id.to_string(),
            proxy_server: proxy_server.clone(),
            wechat_bots,
            org_id,
            admin_user,
        })
    }
    fn renew(&self) -> Result<()> {
        let xx = XxAdmin::new(&self.xx_org_gray_id, self.proxy_server.clone())?;
        let mut d = self.data.write().unwrap();
        *d = xx;
        Ok(())
    }

    pub async fn get(&self) -> Result<State> {
        let s = {
            let data = self.data.read().unwrap();
            data.get_state()
        };
        if let State::Complete(ms) = s.clone() {
            daily_score(
                ms,
                self.wechat_bots.clone(),
                self.org_id,
                &self.admin_user,
                &self.mp,
            )
            .await?;
        }

        match s.clone() {
            State::Broken(_) => self.renew()?,
            State::Complete(_) => self.renew()?,
            _ => {}
        };

        Ok(s)
    }
}

use crate::try_study;
use crate::utils::{get_login_ticket, new_browser};
use anyhow::Result;
use headless_chrome::browser::context::Context;
use headless_chrome::Browser;
use std::ops::Add;
use std::time::Duration;
use tracing::info;

pub struct Xx {
    browser: Browser,
    ctx_id: String,
    ticket: (String, Vec<u8>),
    available_before: chrono::DateTime<chrono::Local>,
}

impl Xx {
    pub fn new(app_caller: &str) -> Result<Self> {
        info!("new Xx");
        let browser = new_browser(&None)?;
        let ctx_id = browser.new_context()?.get_id().to_string().clone();
        let ctx = Context::new(&browser, ctx_id.clone());
        let ticket = get_login_ticket(&ctx, app_caller)?;
        let available_before = chrono::Local::now().add(chrono::Duration::minutes(3));
        info!("new Xx ok {:?} {}", ticket.0, available_before);
        Ok(Self {
            browser,
            ctx_id,
            ticket,
            available_before,
        })
    }

    pub fn is_login(&self) -> Result<bool> {
        info!("is_login");
        let ctx = Context::new(&self.browser, self.ctx_id.clone());
        Ok(ctx
            .get_tabs()?
            .iter()
            .filter(|t| t.get_url().contains("https://www.xuexi.cn/"))
            .any(|tab| {
                tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(3))
                    .is_ok()
            }))
    }
    pub fn get_ticket(&self) -> String {
        self.ticket.0.clone()
    }
    pub fn try_study(&self, news_list: &[String], video_list: &[String]) -> Result<()> {
        info!("study");
        let ctx = Context::new(&self.browser, self.ctx_id.clone());
        try_study(&ctx, news_list, video_list)?;
        Ok(())
    }

    pub fn ping(&self) -> bool {
        info!("ping");
        self.available_before > chrono::Local::now()
            && self.browser.get_version().is_ok()
            && self
                .browser
                .get_tabs()
                .lock()
                .map(|tabs| tabs.len() < 6)
                .expect("获取浏览器标签页失败")
    }
}

use crate::eval::get_user_info;
use crate::try_study;
use crate::utils::{get_login_ticket, get_one_tab, new_browser};
use anyhow::Result;
use headless_chrome::browser::context::Context;
use headless_chrome::Browser;
use std::ops::Add;
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct Xx {
    browser: Browser,
    ctx_id: String,
    ticket: (String, Vec<u8>),
    available_before: chrono::DateTime<chrono::Local>,
    logged_in: bool,
}

impl Xx {
    pub fn new() -> Result<Self> {
        info!("new Xx");
        let browser = new_browser(&None)?;
        let ctx_id = browser.new_context()?.get_id().to_string().clone();
        let ctx = Context::new(&browser, ctx_id.clone());
        let ticket = get_login_ticket(&ctx)?;
        let available_before = chrono::Local::now().add(chrono::Duration::minutes(3));
        info!("new Xx ok {:?} {}", ticket.0, available_before);
        Ok(Self {
            browser,
            ctx_id,
            ticket,
            available_before,
            logged_in: false,
        })
    }

    pub fn check_login(&mut self) -> Result<bool> {
        info!("check_login");
        if self.logged_in {
            return Ok(true);
        }
        let ctx = Context::new(&self.browser, self.ctx_id.clone());
        let r = ctx
            .get_tabs()?
            .iter()
            .filter(|t| t.get_url().contains("https://www.xuexi.cn/"))
            .any(|tab| {
                tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(3))
                    .is_ok()
            });
        self.logged_in = r;
        Ok(r)
    }
    pub fn get_ticket(&self) -> String {
        self.ticket.0.clone()
    }
    pub fn get_user_info(&self) -> Result<String> {
        let ctx = Context::new(&self.browser, self.ctx_id.clone());
        let tab = get_one_tab(&ctx)?;
        get_user_info(&tab)
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
            && !self.logged_in
            && self.browser.get_version().is_ok()
            && self
                .browser
                .get_tabs()
                .lock()
                .map(|tabs| tabs.len() < 6)
                .expect("获取浏览器标签页失败")
    }
}

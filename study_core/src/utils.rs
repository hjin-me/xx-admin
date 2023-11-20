use crate::qrcode::decode_qr;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Local;
use headless_chrome::browser::context::Context;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, instrument, trace};

#[instrument(skip_all)]
pub fn new_browser() -> Result<Browser> {
    trace!("准备启动浏览器");
    let mut rng = thread_rng();
    let w = rng.gen_range(1440..2000);
    let h = rng.gen_range(720..1100);
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .window_size(Some((w, h)))
        // .headless(false)
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(300))
        // .args(vec![OsStr::new("--incognito")])
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;
    Ok(browser)
}

#[instrument(skip_all)]
pub fn reset_tabs(browser: &Context) -> Result<()> {
    // headless 模式 close 有问题，这样将就一下
    let tabs = browser.get_tabs().unwrap();
    for tab in tabs.iter() {
        tab.navigate_to("about:blank")?;
    }
    Ok(())
}
#[instrument(skip_all)]
pub fn get_one_tab(browser: &Context) -> Result<Arc<Tab>> {
    let tabs = browser.get_tabs().unwrap();
    match tabs.into_iter().next() {
        Some(tab) => Ok(tab),
        None => browser
            .new_tab()
            .map_err(|e| anyhow!("创建新标签页失败: {}", e)),
    }
}

#[instrument(skip(ctx))]
pub fn get_login_ticket(ctx: &Context<'_>) -> Result<(String, Vec<u8>)> {
    reset_tabs(ctx)?;
    let tab = get_one_tab(ctx)?;
    tab.navigate_to("https://www.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;

    tab.wait_until_navigated()?;
    let login_btn = tab
        .wait_for_element(".login a.login-icon")
        .map_err(|e| anyhow!("没找到登陆按钮 {}", e))?;

    debug!("点击打开登陆页面");
    login_btn.click()?;
    thread::sleep(Duration::from_secs(2));

    trace!("遍历所有标签页，找到登陆标签");
    let tab = {
        ctx.get_tabs()?
            .clone()
            .into_iter()
            .find(|t| t.get_url().contains("login.html"))
            .ok_or(anyhow!("没有找到登陆标签页"))?
    };
    debug!("等待二维码刷新");
    let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    trace!("获取登陆二维码成功");
    let login_url = decode_qr(&img_data)?;

    Ok((login_url, img_data))
}

#[instrument(skip_all)]
pub fn wait_qr(tab: &Arc<Tab>) -> Result<Vec<u8>> {
    let el = tab
        .wait_for_element(".loginbox-inner")
        .map_err(|e| anyhow!("没找到二维码: {}", e))?;
    thread::sleep(Duration::from_secs(3));
    let viewport = el.get_box_model()?.margin_viewport();
    el.scroll_into_view()?;

    let png_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Png,
        None,
        Some(viewport),
        true,
    )?;
    Ok(png_data)
}
#[instrument(skip_all)]
pub fn get_xuexi_tab(ctx: &Context<'_>) -> Result<Arc<Tab>> {
    let r = ctx
        .get_tabs()?
        .iter()
        .find(|t| t.get_url().contains("https://www.xuexi.cn/"))
        .cloned();
    match r {
        Some(tab) => Ok(tab),
        None => {
            let tab = ctx.new_tab()?;
            tab.navigate_to("https://www.xuexi.cn/")?;
            tab.wait_until_navigated()?;
            Ok(tab)
        }
    }
}

#[instrument(skip_all)]
pub async fn get_news_list() -> Result<Vec<String>> {
    get_some_url("https://www.xuexi.cn/lgdata/1jscb6pu1n2.json")
        .await
        .map_err(|e| anyhow!("请求新闻列表失败: {}", e))
}
#[instrument(skip_all)]
pub async fn get_video_list() -> Result<Vec<String>> {
    get_some_url("https://www.xuexi.cn/lgdata/3o3ufqgl8rsn.json")
        .await
        .map_err(|e| anyhow!("请求视频列表失败: {}", e))
}
#[derive(Deserialize, Debug)]
struct News {
    // #[serde(rename = "publishTime")]
    // publish_time: String,
    #[serde(rename = "auditTime")]
    audit_time: String,
    url: String,
}
#[instrument]
async fn get_some_url(api: &str) -> Result<Vec<String>> {
    let resp = reqwest::get(api)
        .await
        .map_err(|e| anyhow!("请求列表失败: {}", e))?;
    debug!("获取新闻列表 status code {}", resp.status());
    let b = resp.text().await?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let r: Vec<News> = serde_json::from_str(&b)?;
    let mut latest: Vec<String> = r
        .iter()
        .filter(|n| n.audit_time.starts_with(&today))
        .map(|n| n.url.clone())
        .collect();
    let mut rng = thread_rng();
    let shuffle: Vec<String> = r
        .choose_multiple(&mut rng, 30)
        .map(|n| n.url.clone())
        .collect();
    latest.extend(shuffle);
    Ok(latest)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_new_browser() -> Result<()> {
        let b = new_browser()?;
        let browser_tabs = b.get_tabs().lock().unwrap();
        // let mut tabs = vec![];
        for tab in browser_tabs.iter() {
            dbg!(tab.get_target_info()?);
            // if let Some(context_id) = tab.get_browser_context_id()? {
            //     dbg!(context_id);
            // }
        }
        drop(browser_tabs);
        dbg!("context");
        let c = b.new_context()?;
        c.new_tab()?;
        let browser_tabs = b.get_tabs().lock().unwrap();
        for tab in browser_tabs.iter() {
            dbg!(tab.get_target_info()?);
            // if let Some(context_id) = tab.get_browser_context_id()? {
            //     dbg!(context_id);
            // }
        }
        drop(browser_tabs);
        // let t = b.new_tab()?;
        // t.navigate_to("http://www.baidu.com")?;
        panic!("hei");

        // thread::sleep(Duration::from_secs(60));
        // Ok(())
    }
}

#[async_trait]
pub trait UserValidator {
    async fn validate(&self, uid: i64) -> Result<bool>;
}

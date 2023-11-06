use crate::wx::{revoke_msg, send_image_msg, send_text_msg};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[async_trait]
pub trait Fetcher {
    async fn get_score(&self, date: &str) -> Result<MemberScore>;
}

pub struct FetcherImpl {
    login_user: String,
    xx_org_gray_id: String,
    wechat_proxy: String,
    http_client: Client,
    proxy_server: Option<String>,
}

impl FetcherImpl {
    pub fn new(
        login_user: &str,
        xx_org_gray_id: &str,
        wechat_proxy: &str,
        proxy_server: Option<String>,
    ) -> Self {
        let http_client = {
            let b = ClientBuilder::default();
            match proxy_server.as_ref() {
                Some(s) => b
                    .proxy(reqwest::Proxy::all(s.to_owned()).expect("解析 proxy 格式失败"))
                    .build()
                    .expect("初始化 http client 失败"),
                None => b.no_proxy().build().expect("初始化 http client 失败"),
            }
        };
        Self {
            login_user: login_user.to_string(),
            wechat_proxy: wechat_proxy.to_string(),
            http_client,
            xx_org_gray_id: xx_org_gray_id.to_string(),
            proxy_server: proxy_server.clone().map(|s| s.to_string()),
        }
    }
}

#[async_trait]
impl Fetcher for FetcherImpl {
    async fn get_score(&self, date: &str) -> Result<MemberScore> {
        browse_xx(
            &self.http_client,
            &self.login_user,
            &self.wechat_proxy,
            date,
            &self.xx_org_gray_id,
            &self.proxy_server,
        )
        .await
    }
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

async fn browse_xx(
    http_client: &Client,
    login_user: &str,
    wechat_proxy: &str,
    date: &str,
    xx_org_gray_id: &str,
    proxy_server: &Option<String>,
) -> Result<MemberScore> {
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .port(Some(8000))
        .sandbox(false)
        .proxy_server(proxy_server)
        .build()?;
    let browser = Browser::new(launch_options)?;

    let tab = try_navigate_to_xx(&browser, http_client, login_user, wechat_proxy, 20).await?;

    // Run JavaScript in the page
    let yesterday_js = include_str!("yesterday_score.js");
    let body = tab.wait_for_element("body")?;
    let remote_object =
        body.call_js_fn(yesterday_js, vec![date.into(), xx_org_gray_id.into()], true)?;
    let score_result = match remote_object.value {
        Some(serde_json::Value::String(returned_string)) => {
            let v = serde_json::from_str::<MemberScore>(&returned_string)?;
            Ok(v)
        }
        Some(v) => {
            warn!("执行脚本获取数据失败, {:?}", v);
            tokio::time::sleep(Duration::from_secs(60)).await;
            Err(anyhow!("执行脚本获取数据失败"))
        }
        _ => {
            warn!("执行脚本获取数据失败");
            tokio::time::sleep(Duration::from_secs(60)).await;
            Err(anyhow!("执行脚本获取数据失败"))
        }
    }?;

    Ok(score_result)
}
async fn try_navigate_to_xx(
    browser: &Browser,
    client: &Client,
    login_user: &str,
    wechat_proxy: &str,
    times: i8,
) -> Result<Arc<Tab>> {
    for _ in 0..times {
        match navigate_to_xx(browser, client, login_user, wechat_proxy).await {
            Ok(tab) => return Ok(tab),
            Err(e) => {
                warn!("登陆失败了, {:?}", e);
            }
        }
    }
    Err(anyhow!("经过{}次重试，未能成功登陆", times))
}

async fn navigate_to_xx(
    browser: &Browser,
    client: &Client,
    login_user: &str,
    wechat_proxy: &str,
) -> Result<Arc<Tab>> {
    let tabs = browser.get_tabs();

    for x in tabs.lock().unwrap().iter() {
        x.close(false)?;
    }

    let tab = browser.new_tab()?;
    tab.navigate_to("https://study.xuexi.cn/")?;

    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    if tab.get_url().starts_with("https://login.xuexi.cn") {
        info!("未登录，尝试登陆");
        loop_login(client, &tab, login_user, wechat_proxy).await?;
    }

    Ok(tab)
}

async fn loop_login(
    client: &Client,
    tab: &Arc<Tab>,
    login_user: &str,
    wechat_proxy: &str,
) -> Result<()> {
    info!("等待二维码刷新");
    let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    info!("获取登陆二维码成功");
    let (m1, m2) = send_login_msg(client, login_user, &img_data, wechat_proxy).await?;
    let _dms = DropMsg {
        wechat_proxy: wechat_proxy.to_string(),
        client: client.clone(),
        ms: vec![m1, m2],
    };
    info!("发送登陆消息通知");
    let btn = tab.wait_for_element_with_custom_timeout("form button", Duration::from_secs(260))?;
    info!("扫码验证成功，点击确定按钮");
    btn.click()?;
    info!("完成点击登陆按钮");
    tab.wait_for_element("#userName")?;
    info!("完成登陆");
    Ok(())
}

fn wait_qr(tab: &Arc<Tab>) -> Result<Vec<u8>> {
    let el = tab.wait_for_element("iframe")?;
    std::thread::sleep(Duration::from_secs(3));
    let viewport = el.get_box_model()?.margin_viewport();
    let png_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Png,
        None,
        Some(viewport),
        true,
    )?;
    Ok(png_data)
}

async fn send_login_msg(
    client: &Client,
    u: &str,
    img_data: &[u8],
    wechat_proxy: &str,
) -> Result<(String, String)> {
    let before = chrono::Local::now().add(chrono::Duration::minutes(4));

    let m1 = send_image_msg(client, u, img_data, wechat_proxy).await?;
    let m2 = send_text_msg(
        client,
        u,
        &format!("扫码登陆学习强国管理端，{}前效", before.format("%H:%M:%S")),
        wechat_proxy,
    )
    .await?;

    Ok((m1, m2))
}

struct DropMsg {
    client: Client,
    wechat_proxy: String,
    ms: Vec<String>,
}

impl Drop for DropMsg {
    fn drop(&mut self) {
        let ms = self.ms.clone();
        let wp = self.wechat_proxy.clone();
        let c = self.client.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let wp = wp.clone();
                let c = c.clone();
                let _ = revoke_msg(&c, &wp, ms).await;
            });
        });
    }
}

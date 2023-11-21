use anyhow::{anyhow, Result};
use async_trait::async_trait;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, instrument, warn};
use wx::{drop_msg_task, DropMsg, MsgApi, MP};

#[async_trait]
pub trait Fetcher {
    async fn get_score(&self, date: &str) -> Result<MemberScore>;
}

pub struct FetcherImpl {
    login_user: String,
    xx_org_gray_id: String,
    proxy_server: Option<String>,
    mp: MP,
}

impl FetcherImpl {
    pub fn new(
        login_user: &str,
        xx_org_gray_id: &str,
        mp: &MP,
        proxy_server: Option<String>,
    ) -> Self {
        Self {
            login_user: login_user.to_string(),
            mp: mp.clone(),
            xx_org_gray_id: xx_org_gray_id.to_string(),
            proxy_server: proxy_server.clone().map(|s| s.to_string()),
        }
    }
}

#[async_trait]
impl Fetcher for FetcherImpl {
    async fn get_score(&self, date: &str) -> Result<MemberScore> {
        try_browse_xx(
            &self.login_user,
            &self.mp,
            date,
            &self.xx_org_gray_id,
            &self.proxy_server,
            20,
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

#[instrument(skip(mp, proxy_server))]
async fn browse_xx(
    login_user: &str,
    mp: &MP,
    date: &str,
    xx_org_gray_id: &str,
    proxy_server: &Option<String>,
) -> Result<MemberScore> {
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        // .port(Some(8000))
        .sandbox(false)
        .proxy_server(proxy_server)
        .idle_browser_timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;

    let tab = navigate_to_xx(&browser, login_user, mp).await?;

    // Run JavaScript in the page
    let yesterday_js = include_str!("yesterday_score.js");
    let body = tab
        .wait_for_element("body")
        .map_err(|e| anyhow!("获取执行 js 的DOM: {}", e))?;
    let remote_object = body
        .call_js_fn(yesterday_js, vec![date.into(), xx_org_gray_id.into()], true)
        .map_err(|e| anyhow!("执行js脚本失败: {}", e))?;
    let score_result = match remote_object.value {
        Some(serde_json::Value::String(returned_string)) => {
            let v = serde_json::from_str::<MemberScore>(&returned_string)
                .map_err(|e| anyhow!("解析学习强国分数失败: {}: {}", e, returned_string))?;
            Ok(v)
        }
        Some(v) => {
            warn!("执行脚本获取数据失败, {:?}", v);
            tokio::time::sleep(Duration::from_secs(1)).await;
            Err(anyhow!("执行脚本获取数据失败"))
        }
        _ => {
            warn!("执行脚本获取数据失败");
            tokio::time::sleep(Duration::from_secs(1)).await;
            Err(anyhow!("执行脚本获取数据失败"))
        }
    }?;

    Ok(score_result)
}
async fn try_browse_xx(
    login_user: &str,
    mp: &MP,
    date: &str,
    xx_org_gray_id: &str,
    proxy_server: &Option<String>,
    times: i8,
) -> Result<MemberScore> {
    for _ in 0..times {
        match browse_xx(login_user, mp, date, xx_org_gray_id, proxy_server).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                warn!("获取积分失败: {:?}", e);
            }
        }
    }
    Err(anyhow!("经过{}次重试，未能成果获取积分", times))
}
#[instrument(skip(browser, mp))]
async fn navigate_to_xx(browser: &Browser, login_user: &str, mp: &MP) -> Result<Arc<Tab>> {
    let tab = browser
        .new_tab()
        .map_err(|e| anyhow!("创建新标签页失败: {}", e))?;
    tab.navigate_to("https://study.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;

    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    if tab.get_url().starts_with("https://login.xuexi.cn") {
        info!("未登录，尝试登陆");
        loop_login(&tab, login_user, mp).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    info!("登陆成功");

    Ok(tab)
}
#[instrument(skip(tab, mp))]
async fn loop_login(tab: &Arc<Tab>, login_user: &str, mp: &MP) -> Result<()> {
    let tx = drop_msg_task(mp);
    info!("等待二维码刷新");
    let img_data = wait_qr(tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    info!("获取登陆二维码成功");
    // let login_url = study_core::decode_qr(&img_data)?;

    let (m1, m2) = send_login_msg(login_user, &img_data, mp).await?;
    let _dms = DropMsg::new(tx, vec![m1, m2]);
    info!("已发送登陆通知");
    let btn = tab
        .wait_for_element_with_custom_timeout("form button", Duration::from_secs(260))
        .map_err(|e| anyhow!("等待扫码超时:{}", e))?;
    info!("扫码验证成功，点击确定按钮");
    btn.click()?;
    info!("完成点击登陆按钮");
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

async fn send_login_msg<T: MsgApi>(u: &str, img_data: &[u8], mp: &T) -> Result<(String, String)> {
    let before = chrono::Local::now().add(chrono::Duration::minutes(4));
    let m1 = mp.send_image_msg(u, img_data).await?;

    let m2 = mp
        .send_text_msg(
            u,
            &format!(
                "管理员\n学习强国扫码登陆\n{} 前效",
                before.format("%H:%M:%S")
            ),
        )
        .await?;

    Ok((m1, m2))
}

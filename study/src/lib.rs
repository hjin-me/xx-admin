use anyhow::{anyhow, Result};
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::info;
use wx::{DropMsg, MsgApi};

pub struct FetcherImpl {
    login_user: String,
    proxy_server: Option<String>,
}

impl FetcherImpl {
    pub fn new(login_user: &str, proxy_server: Option<String>) -> Self {
        Self {
            login_user: login_user.to_string(),
            proxy_server: proxy_server.clone().map(|s| s.to_string()),
        }
    }
}

async fn browse_xx<T: MsgApi + Clone>(
    mp: &T,
    login_user: &str,
    proxy_server: &Option<String>,
) -> Result<()> {
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .window_size(Some((1920, 1080)))
        .headless(false)
        .port(Some(8000))
        .sandbox(false)
        .proxy_server(proxy_server)
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;

    let _tab = navigate_to_xx(&browser, login_user, mp).await?;

    // Run JavaScript in the page
    // let yesterday_js = include_str!("yesterday_score.js");
    // let body = tab.wait_for_element("body")?;
    // let remote_object =
    //     body.call_js_fn(yesterday_js, vec![date.into(), xx_org_gray_id.into()], true)?;
    // let score_result = match remote_object.value {
    //     Some(serde_json::Value::String(returned_string)) => {
    //         let v = serde_json::from_str::<MemberScore>(&returned_string)?;
    //         Ok(v)
    //     }
    //     Some(v) => {
    //         warn!("执行脚本获取数据失败, {:?}", v);
    //         tokio::time::sleep(Duration::from_secs(60)).await;
    //         Err(anyhow!("执行脚本获取数据失败"))
    //     }
    //     _ => {
    //         warn!("执行脚本获取数据失败");
    //         tokio::time::sleep(Duration::from_secs(60)).await;
    //         Err(anyhow!("执行脚本获取数据失败"))
    //     }
    // }?;

    Ok(())
}

async fn navigate_to_xx<T: MsgApi + Clone>(
    browser: &Browser,
    login_user: &str,
    mp: &T,
) -> Result<Arc<Tab>> {
    let tab = {
        let tabs = browser.get_tabs().lock().unwrap().clone();
        match tabs.into_iter().next() {
            Some(tab) => tab,
            None => browser
                .new_tab()
                .map_err(|e| anyhow!("创建新标签页失败: {}", e))?,
        }
    };
    tab.activate()?;
    tab.navigate_to("https://www.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;

    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    if let Ok(login_btn) = tab.wait_for_element(".login a.login-icon") {
        info!("没有登陆");
        login_btn.click()?;
        time::sleep(Duration::from_secs(2)).await;
        login(browser, login_user, mp).await?
    }
    time::sleep(Duration::from_secs(15)).await;

    Ok(tab)
}

async fn login<T: MsgApi + Clone>(browser: &Browser, login_user: &str, mp: &T) -> Result<()> {
    info!("遍历所有标签页，找到登陆标签");
    let tab = {
        browser
            .get_tabs()
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .find(|t| t.get_url().contains("login.html"))
            .ok_or(anyhow!("没有找到登陆标签页"))?
    };
    info!("等待二维码刷新");
    let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    info!("获取登陆二维码成功");

    info!("滚动一下");
    scroll_to(&tab, 200).await?;
    info!("滚完了");

    let (m1, m2) = send_login_msg(login_user, &img_data, mp).await?;
    let _dms = DropMsg::new(mp, vec![m1, m2]);
    info!("发送登陆消息通知");
    match tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(260)) {
        Ok(_) => {
            info!("扫码验证成功");
        }
        Err(e) => {
            info!("没有登陆, {}", e);
            return Err(anyhow!("没有登陆: {}", e));
        }
    }
    time::sleep(Duration::from_secs(5)).await;
    Ok(())
}

fn wait_qr(tab: &Arc<Tab>) -> Result<Vec<u8>> {
    let el = tab
        .wait_for_element(".loginbox-inner")
        .map_err(|e| anyhow!("没找到二维码: {}", e))?;
    std::thread::sleep(Duration::from_secs(3));
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
async fn send_login_msg<T: MsgApi>(u: &str, img_data: &[u8], mp: &T) -> Result<(String, String)> {
    let before = chrono::Local::now().add(chrono::Duration::minutes(4));
    let m1 = mp.send_image_msg(u, img_data).await?;

    let m2 = mp
        .send_text_msg(
            u,
            &format!("学习强国扫码登陆，{} 前效", before.format("%H:%M:%S")),
        )
        .await?;

    Ok((m1, m2))
}

async fn scroll_to(tab: &Arc<Tab>, to: i64) -> Result<()> {
    let smooth_scroll_js = include_str!("smooth_scroll.js");

    let body = tab
        .wait_for_element("body")
        .map_err(|e| anyhow!("没找到 body: {}", e))?;

    let _remote_object = body.call_js_fn(smooth_scroll_js, vec![to.into()], false)?;
    time::sleep(Duration::from_secs(2)).await;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use wx::MP;
    #[derive(Deserialize)]
    struct Conf {
        corp_id: String,
        corp_secret: String,
        agent_id: i64,
        to_user: String,
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_browser() -> Result<()> {
        tracing_subscriber::fmt::init();
        let conf: Conf = serde_json::from_str(include_str!("../../wx/config.json"))?;
        let mp = MP::new(&conf.corp_id, &conf.corp_secret, conf.agent_id);
        dbg!(browse_xx(&mp, "SongSong", &None).await)?;
        Ok(())
    }
}

use anyhow::{anyhow, Result};
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};

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

async fn browse_xx(
    http_client: &Client,
    login_user: &str,
    wechat_proxy: &str,
    proxy_server: &Option<String>,
) -> Result<()> {
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .headless(false)
        .port(Some(8000))
        .sandbox(false)
        .proxy_server(proxy_server)
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;

    // let _tab = try_navigate_to_xx(&browser, http_client, login_user, wechat_proxy, 20).await?;

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

async fn navigate_to_xx(
    browser: &Browser,
    client: &Client,
    login_user: &str,
    wechat_proxy: &str,
) -> Result<Arc<Tab>> {
    let tab = browser
        .new_tab()
        .map_err(|e| anyhow!("创建新标签页失败: {}", e))?;
    tab.navigate_to("https://www.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;

    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    match tab.wait_for_element(".login a.login-icon") {
        Ok(login_btn) => {
            info!("没有登陆");
            login_btn.click()?;
            loop_login(client, &tab, login_user, "").await?
        }
        Err(e) => {
            info!("已经登陆了");
        }
    }
    // if tab.get_url().starts_with("https://login.xuexi.cn") {
    //     info!("未登录，尝试登陆");
    //     loop_login(client, &tab, login_user, wechat_proxy).await?;
    // }
    time::sleep(Duration::from_secs(15)).await;

    Ok(tab)
}

async fn loop_login(
    client: &Client,
    tab: &Arc<Tab>,
    login_user: &str,
    wechat_proxy: &str,
) -> Result<()> {
    // info!("等待二维码刷新");
    // let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    // info!("获取登陆二维码成功");
    // let (m1, m2) = send_login_msg(client, login_user, &img_data, wechat_proxy).await?;
    // let _dms = DropMsg {
    //     wechat_proxy: wechat_proxy.to_string(),
    //     client: client.clone(),
    //     ms: vec![m1, m2],
    // };
    // info!("发送登陆消息通知");
    // let btn = tab.wait_for_element_with_custom_timeout("form button", Duration::from_secs(260))?;
    // info!("扫码验证成功，点击确定按钮");
    // btn.click()?;
    // info!("完成点击登陆按钮");
    Ok(())
}

fn wait_qr(tab: &Arc<Tab>) -> Result<Vec<u8>> {
    let el = tab.wait_for_element("#ddlogin-iframe")?;
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


#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_browser() -> Result<()> {
        tracing_subscriber::fmt::init();
        let client = ClientBuilder::new().no_proxy().build()?;
        browse_xx(&client, "", "", &None).await?;
        Ok(())
    }
}

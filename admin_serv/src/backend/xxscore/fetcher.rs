use crate::backend::xxscore::get_yesterday;
use crate::state::{MemberScore, StateChange};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{browser, Browser, LaunchOptions, Tab};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;
use study_core::decode_qr;
use study_core::utils::{get_one_tab, new_browser, reset_tabs, Chrome};
use tracing::{debug, error, info, instrument, trace, warn};
use wx::{drop_msg_task, DropMsg, MsgApi, MP};

#[instrument(skip(tx, proxy_server))]
pub async fn browse_xx_admin(
    tx: Sender<StateChange>,
    xx_org_gray_id: &str,
    proxy_server: &Option<String>,
) -> Result<MemberScore> {
    let browser = new_browser(proxy_server.clone())?;
    tx.send(StateChange::Init)?;

    loop {
        let (tab, login_ticket, _) = get_login_ticket(&browser).await?;
        tx.send(StateChange::WaitingLogin(login_ticket.clone()))?;

        if let Ok(_) = waiting_login(&tab, Duration::from_secs(130)) {
            break;
        } else {
            tx.send(StateChange::Init)?;
        }
    }
    tx.send(StateChange::LoggedIn)?;

    let tab = browser
        .get_tabs()?
        .iter()
        .find(|t| t.get_url().contains("https://study.xuexi.cn/admin"))
        .cloned()
        .ok_or(anyhow!("没找到管理员界面标签"))?;

    let date = get_yesterday();
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

#[instrument(skip(ctx))]
async fn get_login_ticket<C: Chrome>(ctx: &C) -> Result<(Arc<Tab>, String, Vec<u8>)> {
    reset_tabs(ctx)?;
    let tab = get_one_tab(ctx)?;
    tab.activate()?;
    tab.navigate_to("https://study.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;
    tab.wait_until_navigated()?;

    debug!("等待二维码刷新");
    let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    trace!("获取登陆二维码成功");
    let login_url = decode_qr(&img_data)?;
    Ok((tab, login_url, img_data))
}
#[instrument(skip_all, level = "trace")]
fn waiting_login(tab: &Arc<Tab>, timeout: Duration) -> Result<bool> {
    let btn = tab
        .wait_for_element_with_custom_timeout("form button", timeout)
        .map_err(|e| anyhow!("等待扫码超时: {}", e))?;
    info!("扫码验证成功，点击确定按钮");
    btn.click()?;
    tab.wait_for_element(".userName")
        .map_err(|e| anyhow!("没找到用户名对应的标签: {}", e))?;
    Ok(true)
}
#[instrument(skip_all)]
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

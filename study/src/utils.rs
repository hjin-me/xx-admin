use crate::qrcode::decode_qr;
use anyhow::{anyhow, Result};
use headless_chrome::browser::context::Context;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, instrument, trace};

#[instrument(skip_all)]
pub fn new_browser(proxy_server: &Option<String>) -> Result<Browser> {
    trace!("准备启动浏览器");
    let mut rng = thread_rng();
    let w = rng.gen_range(1440..2000);
    let h = rng.gen_range(720..1100);
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .window_size(Some((w, h)))
        // .headless(false)
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(300))
        // .args(vec![OsStr::new("--incognito")])
        .proxy_server(proxy_server)
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
pub fn get_login_ticket(ctx: &Context<'_>, app_caller: &str) -> Result<(String, Vec<u8>)> {
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
    let mut app_caller = app_caller.to_string();
    app_caller.extend(form_urlencoded::byte_serialize(login_url.as_bytes()));

    Ok((app_caller, img_data))
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_new_browser() -> Result<()> {
        let b = new_browser(&None)?;
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

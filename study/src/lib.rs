mod eval;

use crate::eval::{get_today_score, get_today_tasks, get_user_info, scroll_to};
use anyhow::{anyhow, Result};
use chrono::Local;
use headless_chrome::browser::context::Context;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::ops::Add;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, instrument, trace, warn};
use wx::{drop_msg_task, DropMsg, MsgApi, MP};

#[instrument(skip_all)]
fn new_browser(proxy_server: &Option<String>) -> Result<Browser> {
    trace!("准备启动浏览器");
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .window_size(Some((1920, 1080)))
        // .headless(false)
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(300))
        .proxy_server(proxy_server)
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;
    Ok(browser)
}

#[instrument(skip_all, fields(user = %login_user))]
pub async fn browse_xx(mp: &MP, login_user: &str, proxy_server: &Option<String>) -> Result<()> {
    let mut browser = new_browser(proxy_server)?;
    let mut ctx = browser.new_context()?;

    info!(user = login_user, "等待用户登陆");
    let mut logined = false;
    let mut nick_name = "".to_string();
    for _ in 0..20 {
        if logined {
            break;
        }
        {
            if ctx.get_tabs().unwrap().len() > 3 {
                drop(browser);
                trace!("哎，关不了 tab，只能关浏览器重启了");
                browser = new_browser(proxy_server)?;
                ctx = browser.new_context()?;
            }
        }

        match try_login(&ctx, login_user, mp).await {
            Ok(n) => {
                logined = true;
                nick_name = n;
            }
            Err(e) => {
                warn!("登陆失败: {:?}", e);
            }
        };
    }
    if !logined {
        error!("经过20次重试，未能登陆");
        mp.send_text_msg(login_user, "登陆失败了，哎")
            .await
            .map_err(|e| anyhow!("发送登陆失败消息失败: {}", e))?;
        return Err(anyhow!("经过20次重试，未能登陆"));
    }
    trace!(nick = nick_name, user = login_user, "登陆成功");
    mp.send_text_msg(login_user, &format!("Hi, {} 学习强国登陆成功", nick_name))
        .await
        .map_err(|e| anyhow!("发送登陆成功消息失败: {}", e))?;

    for _ in 0..2 {
        match try_study(&ctx).await {
            Ok(_) => {
                let n = study_report(&ctx, login_user, mp).await?;
                info!(
                    nick = nick_name,
                    user = login_user,
                    score = n,
                    "今日学习成功"
                );
                break;
            }
            Err(e) => {
                warn!(nick = nick_name, user = login_user, "学习失败: {:?}", e);
            }
        };
    }
    Ok(())
}

#[instrument(skip_all)]
async fn study_report<T: MsgApi + Clone>(
    browser: &Context<'_>,
    login_user: &str,
    mp: &T,
) -> Result<i64> {
    let tab = get_one_tab(browser)?;
    let n = get_today_score(&tab)?;
    trace!(user = login_user, score = n, "发送今日分数");
    mp.send_text_msg(login_user, &format!("今日学习强国分数是：{}", n))
        .await?;
    Ok(n)
}
#[instrument(skip_all)]
async fn try_study(browser: &Context<'_>) -> Result<()> {
    let news_list = get_news_list().await?;
    let video_list = get_video_list().await?;
    let mut news_iter = news_list.iter();
    let mut video_iter = video_list.iter();

    loop {
        let tab = get_one_tab(browser)?;
        let todo_tasks = get_today_tasks(&tab)?;
        if !todo_tasks
            .iter()
            .filter(|e| e.title.as_str() == "我要选读文章" || e.title.as_str() == "我要视听学习")
            .any(|e| e.day_max_score != e.current_score)
        {
            debug!("今日文章和视频任务完成");
            break;
        }
        for task in todo_tasks {
            match task.title.as_str() {
                "我要选读文章" => {
                    if task.day_max_score == task.current_score {
                        debug!("今日阅读任务完成");
                        continue;
                    }
                    info!(
                        "今日阅读分数 {}/{} ",
                        task.current_score, task.day_max_score
                    );
                    if let Some(u) = news_iter.next() {
                        debug!("开始阅读 {}", u);
                        browse_news(browser, u)?;
                    } else {
                        warn!("居然没有文章了，不知道怎么处理");
                        time::sleep(Duration::from_secs(300)).await;
                    }
                }
                "我要视听学习" => {
                    if task.day_max_score == task.current_score {
                        debug!("今日视频任务完成");
                        continue;
                    }
                    info!(
                        "今日视频分数 {}/{} ",
                        task.current_score, task.day_max_score
                    );
                    if let Some(u) = video_iter.next() {
                        debug!("开始观看视频 {}", u);
                        browse_video(browser, u)?;
                    } else {
                        warn!("居然没有视频了，不知道怎么处理");
                        time::sleep(Duration::from_secs(300)).await;
                    }
                }
                _ => {
                    debug!("不知道怎么处理这个任务: {:?}", task);
                }
            }
        }
    }

    Ok(())
}
#[instrument(skip_all)]
async fn try_login(ctx: &Context<'_>, login_user: &str, mp: &MP) -> Result<String> {
    reset_tabs(ctx)?;
    let tab = get_one_tab(ctx)?;
    tab.activate()?;
    tab.navigate_to("https://www.xuexi.cn/")
        .map_err(|e| anyhow!("打开学习页面失败: {}", e))?;

    tab.wait_until_navigated()?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    if let Ok(login_btn) = tab.wait_for_element(".login a.login-icon") {
        debug!("没有登陆");
        login_btn.click()?;
        time::sleep(Duration::from_secs(2)).await;
        login(ctx, login_user, mp).await?
    }
    time::sleep(Duration::from_secs(5)).await;
    let nick_name = get_user_info(&tab)?;
    Ok(nick_name)
}
#[instrument(skip_all)]
async fn login(browser: &Context<'_>, login_user: &str, mp: &MP) -> Result<()> {
    let tx = drop_msg_task(mp);
    trace!("遍历所有标签页，找到登陆标签");
    let tab = {
        browser
            .get_tabs()
            .unwrap()
            .clone()
            .into_iter()
            .find(|t| t.get_url().contains("login.html"))
            .ok_or(anyhow!("没有找到登陆标签页"))?
    };
    debug!("等待二维码刷新");
    let img_data = wait_qr(&tab).map_err(|e| anyhow!("wait qr error: {:?}", e))?;
    trace!("获取登陆二维码成功");

    let (m1, m2) = send_login_msg(login_user, &img_data, mp).await?;
    let _dms = DropMsg::new(tx, vec![m1, m2]);
    trace!("发送登陆消息通知");
    match tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(260)) {
        Ok(_) => {
            info!("扫码登陆成功");
        }
        Err(e) => {
            info!("没有登陆, {}", e);
            return Err(anyhow!("没有登陆: {}", e));
        }
    }
    Ok(())
}

#[instrument(skip_all)]
fn wait_qr(tab: &Arc<Tab>) -> Result<Vec<u8>> {
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
async fn send_login_msg<T: MsgApi>(u: &str, img_data: &[u8], mp: &T) -> Result<(String, String)> {
    let before = Local::now().add(chrono::Duration::minutes(4));
    let m1 = mp.send_image_msg(u, img_data).await?;

    let m2 = mp
        .send_text_msg(
            u,
            &format!("学习强国扫码登陆，{} 前效", before.format("%H:%M:%S")),
        )
        .await?;

    Ok((m1, m2))
}

#[instrument(skip(browser))]
fn browse_news(browser: &Context<'_>, url: &str) -> Result<()> {
    let tab = get_one_tab(browser)?;
    tab.activate()?;
    tab.navigate_to(url)?;
    thread::sleep(Duration::from_secs(10));
    scroll_to(&tab, 394)?;
    let s = {
        let mut rng = thread_rng();
        rng.gen_range(80..110)
    };
    debug!("阅读文章 {} 秒", s);
    thread::sleep(Duration::from_secs(s / 2));
    scroll_to(&tab, 1000)?;
    thread::sleep(Duration::from_secs(s / 2));
    scroll_to(&tab, 3000)?;
    thread::sleep(Duration::from_secs(10));
    scroll_to(&tab, 0)?;
    // headless 模式下，close 没有反应？
    // tab.close(false)?;
    Ok(())
}
#[instrument(skip(browser))]
fn browse_video(browser: &Context<'_>, url: &str) -> Result<()> {
    let tab = get_one_tab(browser)?;
    tab.activate()?;
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    thread::sleep(Duration::from_secs(1));
    scroll_to(&tab, 394)?;
    let play_js = include_str!("play.js");
    tab.evaluate(play_js, false)?;
    let s = {
        let mut rng = thread_rng();
        rng.gen_range(130..260)
    };
    debug!("观看视频 {} 秒", s);
    thread::sleep(Duration::from_secs(s / 2));
    scroll_to(&tab, 500)?;
    thread::sleep(Duration::from_secs(s / 2));
    scroll_to(&tab, 300)?;
    // tab.close(false)?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct News {
    // #[serde(rename = "publishTime")]
    // publish_time: String,
    #[serde(rename = "auditTime")]
    audit_time: String,
    url: String,
}

#[instrument(skip_all)]
async fn get_news_list() -> Result<Vec<String>> {
    get_news_url("https://www.xuexi.cn/lgdata/1jscb6pu1n2.json")
        .await
        .map_err(|e| anyhow!("请求新闻列表失败: {}", e))
}
#[instrument(skip_all)]
async fn get_video_list() -> Result<Vec<String>> {
    get_news_url("https://www.xuexi.cn/lgdata/3o3ufqgl8rsn.json")
        .await
        .map_err(|e| anyhow!("请求视频列表失败: {}", e))
}
#[instrument]
async fn get_news_url(api: &str) -> Result<Vec<String>> {
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
#[instrument(skip_all)]
fn reset_tabs(browser: &Context) -> Result<()> {
    // headless 模式 close 有问题，这样将就一下
    let tabs = browser.get_tabs().unwrap();
    for tab in tabs.iter() {
        tab.navigate_to("about:blank")?;
    }
    Ok(())
}
#[instrument(skip_all)]
fn get_one_tab(browser: &Context) -> Result<Arc<Tab>> {
    let tabs = browser.get_tabs().unwrap();
    match tabs.into_iter().next() {
        Some(tab) => Ok(tab),
        None => browser
            .new_tab()
            .map_err(|e| anyhow!("创建新标签页失败: {}", e)),
    }
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
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_browser() -> Result<()> {
        tracing_subscriber::fmt::init();
        let conf: Conf = serde_json::from_str(include_str!("../../wx/config.json"))?;
        let mp = MP::new(&conf.corp_id, &conf.corp_secret, conf.agent_id);
        dbg!(browse_xx(&mp, "SongSong", &None).await)?;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_browser_close() -> Result<()> {
        tracing_subscriber::fmt::init();
        let b = new_browser(&None)?;
        let c = b.new_context()?;
        info!("open new tab");
        let tab = c.new_tab()?;
        time::sleep(Duration::from_secs(10)).await;
        info!("will close tab");
        drop(c);
        tab.close(false)?;
        info!("after close");
        time::sleep(Duration::from_secs(10)).await;
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_news_video() -> Result<()> {
        tracing_subscriber::fmt::init();
        let r = dbg!(get_news_list().await?);
        info!("news: {:?}", r);
        let r = dbg!(get_video_list().await?);
        info!("video: {:?}", r);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_headless_close() -> Result<()> {
        tracing_subscriber::fmt::init();
        let browser = new_browser(&None)?;

        let tab = browser.new_tab()?;
        {
            let tabs = browser.get_tabs().lock().unwrap();
            info!("tab 数量 {}", tabs.iter().count());
        }
        info!("{:?}", tab.get_target_info()?);
        info!("关闭标签页");
        tab.close(false)?;
        // sleep(Duration::from_secs(3)).await;
        {
            let tabs = browser.get_tabs().lock().unwrap();
            info!("tab 数量 {}", tabs.iter().count());
        }
        Ok(())
    }
}

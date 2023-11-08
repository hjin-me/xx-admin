use anyhow::{anyhow, Result};
use chrono::Local;
use headless_chrome::browser::default_executable;
use headless_chrome::protocol::cdp::Page;
use headless_chrome::{Browser, LaunchOptions, Tab};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};
use wx::{DropMsg, MsgApi};

fn new_browser(proxy_server: &Option<String>) -> Result<Browser> {
    let proxy_server = proxy_server.as_ref().map(|s| s.as_str());
    let launch_options = LaunchOptions::default_builder()
        .path(Some(default_executable().map_err(|e| anyhow!(e))?))
        .window_size(Some((1920, 1080)))
        .port(Some(8000))
        // .headless(false)
        .sandbox(false)
        .idle_browser_timeout(Duration::from_secs(300))
        .proxy_server(proxy_server)
        .build()
        .map_err(|e| anyhow!("构造 Chrome 启动参数失败: {}", e))?;
    let browser = Browser::new(launch_options).map_err(|e| anyhow!("启动浏览器失败: {}", e))?;
    Ok(browser)
}

async fn browse_xx<T: MsgApi + Clone>(
    mp: &T,
    login_user: &str,
    proxy_server: &Option<String>,
) -> Result<()> {
    let browser = new_browser(proxy_server)?;

    let _tab = navigate_to_xx(&browser, login_user, mp).await?;

    Ok(())
}

async fn navigate_to_xx<T: MsgApi + Clone>(
    browser: &Browser,
    login_user: &str,
    mp: &T,
) -> Result<()> {
    let tab = get_one_tab(browser)?;
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
    time::sleep(Duration::from_secs(5)).await;

    let news_list = get_news_list().await?;
    let video_list = get_video_list().await?;
    let mut news_iter = news_list.iter();
    let mut video_iter = video_list.iter();

    loop {
        let todo_tasks = get_today_tasks(&tab).await?;
        if todo_tasks
            .iter()
            .filter(|e| e.title.as_str() == "我要选读文章" || e.title.as_str() == "我要视听学习")
            .find(|e| e.day_max_score != e.current_score)
            .is_none()
        {
            info!("今日文章和视频任务完成");
            break;
        }
        for task in todo_tasks {
            match task.title.as_str() {
                "我要选读文章" => {
                    if task.day_max_score == task.current_score {
                        info!("今日阅读任务完成");
                        continue;
                    }
                    info!(
                        "今日阅读分数 {}/{} ",
                        task.current_score, task.day_max_score
                    );
                    if let Some(u) = news_iter.next() {
                        info!("开始阅读 {}", u);
                        browse_news(&browser, u).await?;
                    }
                }
                "我要视听学习" => {
                    if task.day_max_score == task.current_score {
                        info!("今日视频任务完成");
                        continue;
                    }
                    info!(
                        "今日视频分数 {}/{} ",
                        task.current_score, task.day_max_score
                    );
                    if let Some(u) = video_iter.next() {
                        info!("开始观看视频 {}", u);
                        browse_video(&browser, u).await?;
                    }
                }
                _ => {
                    info!("不知道怎么处理这个任务: {:?}", task);
                }
            }
        }
    }
    study_report(browser, login_user, mp).await?;

    Ok(())
}

async fn study_report<T: MsgApi + Clone>(
    browser: &Browser,
    login_user: &str,
    mp: &T,
) -> Result<()> {
    let n = get_today_score(browser).await?;
    info!("发送今日分数");
    mp.send_text_msg(login_user, &format!("今日学习强国分数是：{}", n))
        .await?;
    Ok(())
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
    info!("页面滚动一下");
    let smooth_scroll_js = include_str!("smooth_scroll.js");

    let body = tab
        .wait_for_element("body")
        .map_err(|e| anyhow!("没找到 body: {}", e))?;

    let _remote_object = body.call_js_fn(smooth_scroll_js, vec![to.into()], false)?;
    time::sleep(Duration::from_secs(2)).await;
    Ok(())
}

async fn browse_news(browser: &Browser, url: &str) -> Result<()> {
    let tab = get_one_tab(browser)?;
    tab.activate()?;
    tab.navigate_to(url)?;
    time::sleep(Duration::from_secs(10)).await;
    scroll_to(&tab, 394).await?;
    let mut rng = thread_rng();
    let s = rng.gen_range(80..110);
    info!("阅读文章 {} 秒", s);
    time::sleep(Duration::from_secs(s / 2)).await;
    scroll_to(&tab, 1000).await?;
    time::sleep(Duration::from_secs(s / 2)).await;
    scroll_to(&tab, 3000).await?;
    time::sleep(Duration::from_secs(10)).await;
    scroll_to(&tab, 0).await?;
    // headless 模式下，close 没有反应？
    // tab.close(false)?;
    Ok(())
}
async fn browse_video(browser: &Browser, url: &str) -> Result<()> {
    let tab = get_one_tab(browser)?;
    tab.activate()?;
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    time::sleep(Duration::from_secs(1)).await;
    scroll_to(&tab, 394).await?;
    let play_js = include_str!("play.js");
    tab.evaluate(play_js, false)?;

    let mut rng = thread_rng();
    let s = rng.gen_range(10..20);
    info!("观看视频 {} 秒", s);
    time::sleep(Duration::from_secs(s / 2)).await;
    scroll_to(&tab, 500).await?;
    time::sleep(Duration::from_secs(s / 2)).await;
    scroll_to(&tab, 300).await?;
    // tab.close(false)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct TodayTask {
    #[serde(rename = "displayRuleId")]
    pub display_rule_id: String,
    pub title: String,
    pub sort: i64,
    #[serde(rename = "currentScore")]
    pub current_score: i64,
    #[serde(rename = "dayMaxScore")]
    pub day_max_score: i64,
    #[serde(rename = "taskCode")]
    pub task_code: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    #[serde(rename = "userId")]
    pub user_id: i64,
    #[serde(rename = "inBlackList")]
    pub in_black_list: bool,
    #[serde(rename = "totalScore")]
    pub total_score: i64,
    #[serde(rename = "taskProgress")]
    pub task_progress: Vec<TodayTask>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TodayScoreRoot {
    pub data: Data,
}
async fn get_today_score(browser: &Browser) -> Result<i64> {
    let tab = get_one_tab(browser)?;

    let js = include_str!("today_score.js");
    let remote_obj = tab.evaluate(js, true)?;
    let score_result = match remote_obj.value {
        Some(serde_json::Value::Number(returned_num)) => {
            let v = returned_num.as_i64().unwrap();
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
    info!("今天学习总分为 {:?}", score_result);
    Ok(score_result)
}
async fn get_today_tasks(tab: &Arc<Tab>) -> Result<Vec<TodayTask>> {
    info!("获取今日的学习任务");
    let js = include_str!("today_task.js");
    let remote_obj = tab.evaluate(js, true)?;
    let score_result = match remote_obj.value {
        Some(serde_json::Value::String(returned_string)) => {
            let v = serde_json::from_str::<TodayScoreRoot>(&returned_string)?;
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
    info!("今天学习任务的进度是 {:?}", score_result);
    Ok(score_result.data.task_progress)
}

#[derive(Deserialize, Debug)]
struct News {
    // #[serde(rename = "publishTime")]
    // publish_time: String,
    #[serde(rename = "auditTime")]
    audit_time: String,
    url: String,
}

async fn get_news_list() -> Result<Vec<String>> {
    get_news_url("https://www.xuexi.cn/lgdata/1jscb6pu1n2.json")
        .await
        .map_err(|e| anyhow!("请求新闻列表失败: {}", e))
}
async fn get_video_list() -> Result<Vec<String>> {
    get_news_url("https://www.xuexi.cn/lgdata/3o3ufqgl8rsn.json")
        .await
        .map_err(|e| anyhow!("请求视频列表失败: {}", e))
}
async fn get_news_url(api: &str) -> Result<Vec<String>> {
    let resp = reqwest::get(api)
        .await
        .map_err(|e| anyhow!("请求列表失败: {}", e))?;
    info!("获取新闻列表 status code {}", resp.status());
    let b = resp.text().await?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let r: Vec<News> = serde_json::from_str(&b)?;
    let mut latest: Vec<String> = r
        .iter()
        .filter(|n| n.audit_time.starts_with(&today))
        .map(|n| n.url.clone())
        .collect();
    let mut rng = rand::thread_rng();
    let shuffle: Vec<String> = r
        .choose_multiple(&mut rng, 10)
        .map(|n| n.url.clone())
        .collect();
    latest.extend(shuffle);
    Ok(latest)
}

fn get_one_tab(browser: &Browser) -> Result<Arc<Tab>> {
    let tabs = browser.get_tabs().lock().unwrap().clone();
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
    async fn test_news_video() -> Result<()> {
        tracing_subscriber::fmt::init();
        let r = dbg!(get_news_list().await?);
        info!("news: {:?}", r);
        let r = dbg!(get_video_list().await?);
        info!("video: {:?}", r);
        Ok(())
    }
}

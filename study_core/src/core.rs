use crate::eval::{get_today_score, get_today_tasks, get_user_info, scroll_to};
pub use crate::qrcode::*;
pub use crate::state::*;
use crate::utils::{
    get_login_ticket, get_news_list, get_one_tab, get_video_list, get_xuexi_tab, new_browser,
};
pub use crate::xx::Xx;
use anyhow::{anyhow, Result};
use headless_chrome::browser::context::Context;
use rand::{thread_rng, Rng};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};

#[instrument(skip_all)]
pub async fn new_xx_task_bg(tx: Sender<StateChange>) -> Result<()> {
    let browser = new_browser()?;
    let ctx = browser.new_context()?;
    tx.send(StateChange::Init)?;

    let login_ticket = get_login_ticket(&ctx)?;
    tx.send(StateChange::WaitingLogin(login_ticket.0.clone()))?;

    waiting_login(&ctx, Duration::from_secs(130)).await?;

    let nick_name = {
        let tab = get_xuexi_tab(&ctx)?;
        get_user_info(&tab)?
    };
    tx.send(StateChange::LoggedIn(nick_name.clone()))?;

    let news_list = get_news_list().await?;
    let video_list = get_video_list().await?;

    tx.send(StateChange::StartLearn)?;
    try_study(&ctx, tx.clone(), &nick_name, &news_list, &video_list)?;

    let n = {
        let tab = get_xuexi_tab(&ctx)?;
        get_today_score(&tab)?
    };
    tx.send(StateChange::Complete((nick_name, n)))?;
    Ok(())
}

#[instrument(skip_all)]
pub fn try_study(
    browser: &Context<'_>,
    tx: Sender<StateChange>,
    nick_name: &str,
    news_list: &[String],
    video_list: &[String],
) -> Result<()> {
    let mut news_iter = news_list.iter();
    let mut video_iter = video_list.iter();

    loop {
        let tab = get_one_tab(browser)?;
        let todo_tasks = get_today_tasks(&tab)?;
        tx.send(StateChange::LearnLog((
            nick_name.to_string(),
            todo_tasks
                .iter()
                .map(|t| (t.title.clone(), t.current_score, t.day_max_score))
                .collect(),
        )))?;
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
                        // thread::sleep(Duration::from_secs(300));
                        return Ok(());
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
                        // thread::sleep(Duration::from_secs(300));
                        return Ok(());
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

#[instrument(skip(browser))]
pub fn browse_news(browser: &Context<'_>, url: &str) -> Result<()> {
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
pub fn browse_video(browser: &Context<'_>, url: &str) -> Result<()> {
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

#[instrument(skip(ctx))]
async fn waiting_login(ctx: &Context<'_>, timeout: Duration) -> Result<()> {
    let check = async {
        loop {
            match check_login(ctx) {
                Ok(b) => {
                    if b {
                        break;
                    } else {
                        debug!("还没登陆");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
                Err(e) => {
                    error!("判断登陆状态失败: {}", e);
                }
            }
        }
    };
    tokio::select! {
        _ = check => {
            return Ok(())
        },
        _ = tokio::time::sleep(timeout) => {
            warn!("等待登陆超时");
            return Err(anyhow!("等待登陆超时"))
        },
    }
}
#[instrument(skip(ctx))]
fn check_login(ctx: &Context<'_>) -> Result<bool> {
    Ok(ctx
        .get_tabs()?
        .iter()
        .filter(|t| t.get_url().contains("https://www.xuexi.cn/"))
        .any(|tab| {
            tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(3))
                .is_ok()
        }))
}
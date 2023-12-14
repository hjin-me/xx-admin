use crate::eval::{get_today_score, get_today_tasks, get_user_info, scroll_to};
pub use crate::qrcode::*;
pub use crate::state::*;
use crate::utils::{
    get_login_ticket, get_news_list, get_one_tab, get_video_list, get_xuexi_tab, new_browser,
    Chrome, UserValidator,
};
pub use crate::xx::Xx;
use anyhow::{anyhow, Result};
use rand::{thread_rng, Rng};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, instrument, trace, warn};

#[instrument(skip_all)]
pub async fn new_xx_task_bg<T: UserValidator>(
    tx: Sender<StateChange>,
    validator: T,
    proxy_server: Option<String>,
) -> Result<()> {
    let browser = new_browser(proxy_server)?;
    tx.send(StateChange::Init)?;

    loop {
        let login_ticket = get_login_ticket(&browser)?;
        tx.send(StateChange::WaitingLogin(login_ticket.0.clone()))?;

        if let Ok(()) = waiting_login(&browser, Duration::from_secs(130)).await {
            break;
        } else {
            tx.send(StateChange::Init)?;
        }
    }

    let user_info = {
        let tab = get_xuexi_tab(&browser)?;
        get_user_info(&tab)?
    };
    // 白名单，黑名单检查
    if !validator.validate(user_info.uid).await? {
        tx.send(StateChange::BrowserClosed(anyhow!("登陆异常")))?;
        return Ok(());
    }
    tx.send(StateChange::LoggedIn(user_info.clone()))?;

    let news_list = get_news_list().await?;
    let video_list = get_video_list().await?;

    tx.send(StateChange::StartLearn)?;
    let n = study_and_summarize(&browser, tx.clone(), &user_info, &news_list, &video_list)?;
    tx.send(StateChange::Complete((user_info.nick, n)))?;
    Ok(())
}

#[instrument(skip_all, fields(nick_name = user_info.nick, uid = user_info.uid))]
fn study_and_summarize<C: Chrome>(
    ctx: &C,
    tx: Sender<StateChange>,
    user_info: &UserInfo,
    news_list: &[String],
    video_list: &[String],
) -> Result<i64> {
    try_study(ctx, tx.clone(), &user_info.nick, &news_list, &video_list)?;

    let n = {
        let tab = get_xuexi_tab(ctx)?;
        get_today_score(&tab)?
    };
    debug!(
        nick_name = &user_info.nick,
        uid = &user_info.uid,
        "今天学习总分为[{}] {}",
        user_info.nick,
        n
    );
    Ok(n)
}

#[instrument(skip_all, fields(nick_name = nick_name))]
fn try_study<C: Chrome>(
    browser: &C,
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
pub fn browse_news<C: Chrome>(browser: &C, url: &str) -> Result<()> {
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
pub fn browse_video<C: Chrome>(browser: &C, url: &str) -> Result<()> {
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
async fn waiting_login<C: Chrome>(ctx: &C, timeout: Duration) -> Result<()> {
    let check = async {
        loop {
            match check_login(ctx) {
                Ok(b) => {
                    if b {
                        break;
                    } else {
                        trace!("还没登陆");
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
            return Err(anyhow!("等待登陆超时"))
        },
    }
}
fn check_login<C: Chrome>(ctx: &C) -> Result<bool> {
    Ok(ctx
        .get_tabs()?
        .iter()
        .filter(|t| t.get_url().contains("https://www.xuexi.cn/"))
        .any(|tab| {
            tab.wait_for_element_with_custom_timeout(".logged-text", Duration::from_secs(3))
                .is_ok()
        }))
}

#[cfg(test)]
mod test {
    use super::*;
    use async_trait::async_trait;
    use std::thread::spawn;
    use tokio::time::sleep;

    #[derive(Clone)]
    struct MockUV {}

    #[async_trait]
    impl UserValidator for MockUV {
        async fn validate(&self, _: i64) -> Result<bool> {
            Ok(true)
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multi_browser() -> Result<()> {
        tracing_subscriber::fmt::init();
        let (tx, rx) = std::sync::mpsc::channel();
        let _h1 = spawn(move || {
            info!("h1");
            tokio::runtime::Runtime::new().unwrap().spawn(async {
                info!("h1 browser");
                _ = new_xx_task_bg(tx, MockUV {}, None).await;
            });
            _ = spawn(move || for _ in rx.iter() {}).join();
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let _h2 = spawn(move || {
            info!("h2");
            tokio::runtime::Runtime::new().unwrap().spawn(async {
                info!("h2 browser");
                _ = new_xx_task_bg(tx, MockUV {}, None).await;
            });
            _ = spawn(move || for _ in rx.iter() {}).join();
        });
        sleep(Duration::from_secs(120)).await;
        // _ = h1.join();
        // _ = h2.join();
        Ok(())
    }
    #[test]
    fn test_close() {
        // TODO 等他们解决关闭标签页的问题
        let browser = new_browser(None).unwrap();
        let tab = get_one_tab(&browser).unwrap();
        tab.close(false).unwrap();
    }
}

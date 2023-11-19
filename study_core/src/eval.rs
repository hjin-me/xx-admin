use anyhow::{anyhow, Result};
use headless_chrome::Tab;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, instrument, trace, warn};

#[derive(Deserialize, Debug)]
pub struct UserInfo {
    // uid: i64,
    pub nick: String,
    // avatarMediaUrl: String,
}

#[derive(Deserialize, Debug)]
struct UserInfoResp {
    pub data: UserInfo,
}

#[instrument(skip(tab))]
pub fn get_user_info(tab: &Arc<Tab>) -> Result<String> {
    trace!("获取当前用户名");
    let js = include_str!("info.js");
    let remote_obj = tab.evaluate(js, true)?;
    let result = match remote_obj.value {
        Some(serde_json::Value::String(returned_string)) => {
            let v = serde_json::from_str::<UserInfoResp>(&returned_string)?;
            Ok(v)
        }
        Some(v) => {
            warn!("执行脚本获取数据失败, {:?}", v);
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
        _ => {
            warn!("执行脚本获取数据失败");
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
    }?;
    Ok(result.data.nick)
}

#[instrument(skip(tab))]
pub fn scroll_to(tab: &Arc<Tab>, to: i64) -> Result<()> {
    trace!("页面滚动一下");
    let smooth_scroll_js = include_str!("smooth_scroll.js");

    let body = tab
        .wait_for_element("body")
        .map_err(|e| anyhow!("没找到 body: {}", e))?;

    let _remote_object = body.call_js_fn(smooth_scroll_js, vec![to.into()], false)?;
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

#[instrument(skip(tab))]
pub fn get_today_score(tab: &Arc<Tab>) -> Result<i64> {
    let js = include_str!("today_score.js");
    let remote_obj = tab.evaluate(js, true)?;
    let score_result = match remote_obj.value {
        Some(serde_json::Value::Number(returned_num)) => {
            let v = returned_num.as_i64().unwrap();
            Ok(v)
        }
        Some(v) => {
            warn!("执行脚本获取数据失败, {:?}", v);
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
        _ => {
            warn!("执行脚本获取数据失败");
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
    }?;
    debug!("今天学习总分为 {:?}", score_result);
    Ok(score_result)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TodayTask {
    #[serde(rename = "displayRuleId")]
    display_rule_id: String,
    pub title: String,
    sort: i64,
    #[serde(rename = "currentScore")]
    pub current_score: i64,
    #[serde(rename = "dayMaxScore")]
    pub day_max_score: i64,
    #[serde(rename = "taskCode")]
    task_code: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    #[serde(rename = "userId")]
    user_id: i64,
    #[serde(rename = "inBlackList")]
    in_black_list: bool,
    #[serde(rename = "totalScore")]
    total_score: i64,
    #[serde(rename = "taskProgress")]
    task_progress: Vec<TodayTask>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TodayScoreRoot {
    data: Data,
}
#[instrument(skip(tab))]
pub fn get_today_tasks(tab: &Arc<Tab>) -> Result<Vec<TodayTask>> {
    trace!("获取今日的学习任务");
    let js = include_str!("today_task.js");
    let remote_obj = tab.evaluate(js, true)?;
    let score_result = match remote_obj.value {
        Some(serde_json::Value::String(returned_string)) => {
            let v = serde_json::from_str::<TodayScoreRoot>(&returned_string)?;
            Ok(v)
        }
        Some(v) => {
            warn!("执行脚本获取数据失败, {:?}", v);
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
        _ => {
            warn!("执行脚本获取数据失败");
            thread::sleep(Duration::from_secs(1));
            Err(anyhow!("执行脚本获取数据失败"))
        }
    }?;
    debug!("今天学习任务的进度是 {:?}", score_result);
    Ok(score_result.data.task_progress)
}

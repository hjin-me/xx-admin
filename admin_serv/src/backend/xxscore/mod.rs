pub mod fetcher;
mod xx;
use crate::state::{Member, MemberScore};
use anyhow::{anyhow, Result};
use std::ops::Sub;
use tracing::{info, instrument};
use wx::MsgApi;
pub use xx::XxAdmin;

#[instrument(skip_all)]
pub async fn daily_score<T: MsgApi>(
    mut score: MemberScore,
    wechat_bots: Vec<String>,
    org_id: u64,
    admin_user: &str,
    mp: &T,
) -> Result<()> {
    score.data.sort_by(|a, b| {
        b.range_real_score
            .partial_cmp(&a.range_real_score)
            .expect("分数比较失败")
    });

    let inactive_count = score.data.iter().filter(|a| a.range_real_score < 1).count();
    let mut grinds = score
        .data
        .iter()
        .filter(|a| a.range_real_score > 34)
        .map(|a| a.user_name.clone())
        .collect::<Vec<String>>();
    grinds.truncate(20);

    // 解析组织的排名
    let org_rank_msg = match score.organization_rank.iter().find(|a| a.org_id == org_id) {
        Some(a) => {
            if a.rank != 1 {
                format!(
                    r#"**园区排名 <font color="info">{}</font>名**, 平均分{}, <font color="comment">落后{}分</font>"#,
                    a.rank, a.avg_score, a.pre_diff_score
                )
            } else {
                r#"# 园区排名 <font color="info">第一</font>**"#.to_string()
            }
        }
        None => "".to_string(),
    };
    let encourage_msg = if inactive_count > 0 {
        format!("{}位同学未完成学习任务。", inactive_count)
    } else {
        "".to_string()
    };

    let grind_msg = if !grinds.is_empty() {
        format!(
            r#"**当日学霸**
> {}"#,
            grinds
                .iter()
                .map(|s| format!("<font color=\"info\">{}</font>", s))
                .collect::<Vec<_>>()
                .join("，")
        )
    } else {
        "".to_string()
    };
    let msg = format!(
        r#"**{} 学习积分情况**

{}

{}

{}"#,
        score.date.clone(),
        grind_msg,
        org_rank_msg,
        encourage_msg
    );

    for bot in wechat_bots {
        mp.send_bot_msg(&msg, &bot)
            .await
            .map_err(|e| anyhow!("发送消息给群机器人失败: {}", e))?;
    }
    // 发送全量汇总信息给管理员
    total_notice(mp, &score.date, score.data, admin_user)
        .await
        .map_err(|e| anyhow!("发送消息给管理员失败: {}", e))?;

    Ok(())
}

async fn total_notice<T: MsgApi>(
    mp: &T,
    date: &str,
    ms: Vec<Member>,
    admin_user: &str,
) -> Result<()> {
    let msg = ms
        .iter()
        .filter(|m| m.range_real_score > 0)
        .map(|m| {
            if m.range_real_score < 25 {
                format!(
                    "> {}: <font color=\"warning\">{}</font>",
                    m.user_name, m.range_real_score
                )
            } else if m.range_real_score < 35 {
                format!("> {}: {}", m.user_name, m.range_real_score)
            } else {
                format!(
                    "> {}: <font color=\"info\">{}</font>",
                    m.user_name, m.range_real_score
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    info!("今日统计结果，{}", msg);

    let inactive_count = ms.iter().filter(|m| m.range_real_score < 1).count();
    mp.send_markdown_msg(
        admin_user,
        &format!(
            "**{} 学习强国积分情况**\n{}\n\n{} 人未学习",
            date, msg, inactive_count
        ),
    )
    .await?;
    Ok(())
}

pub fn get_yesterday() -> String {
    let now = chrono::Local::now().sub(chrono::Duration::days(1));
    now.format("%Y%m%d").to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::config::AdminConfig;
    use crate::state::MemberScore;
    use async_trait::async_trait;
    use wx::MP;

    struct MockFetcher {
        data: MemberScore,
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_cmd() -> Result<()> {
        tracing_subscriber::fmt::init();

        let conf_str = include_str!("../../../config.toml");
        let c = toml::from_str::<AdminConfig>(conf_str)?;

        // let j: MemberScore =
        //     serde_json::from_str(&fs::read_to_string("./src/xxscore/test.json").await?)?;

        // let xx_fetcher = MockFetcher { data: j };
        let mp = MP::new(&c.corp_id, &c.corp_secret, c.agent_id);

        daily_score(
            MemberScore::default(),
            c.notice_bot.iter().map(|s| s.as_str()).collect(),
            c.org_id,
            c.admin_user.as_str(),
            &mp,
        )
        .await
    }
}

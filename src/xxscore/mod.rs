pub mod fetcher;

use crate::wx::send_msg_to_bot;
use crate::xxscore::fetcher::Fetcher;
use anyhow::Result;
use reqwest::Client;
use std::ops::Sub;
// (uid, name, score, department)
pub type MemberScore = (u64, String, u64, String);

pub async fn daily_score<F: Fetcher>(
    date: &str,
    f: &F,
    wechat_bots: Vec<&str>,
    org_id: u64,
) -> Result<()> {
    let mut score = f.get_score(date).await?;
    let mut input: Vec<MemberScore> = vec![];
    for item in &score.data {
        input.push((
            item.user_id,
            item.user_name.clone(),
            item.range_real_score,
            item.dept_names.clone(),
        ))
    }
    score.data.sort_by(|a, b| {
        b.range_real_score
            .partial_cmp(&a.range_real_score)
            .expect("分数比较失败")
    });

    let inactive_count = score.data.iter().filter(|a| a.range_real_score < 1).count();
    let mut grinds = score
        .data
        .iter()
        .filter(|a| a.range_real_score > 35)
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
        format!(
            "{}位同学未完成学习任务，学习不积极，思想有问题。",
            inactive_count
        )
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
        date, grind_msg, org_rank_msg, encourage_msg
    );

    for bot in wechat_bots {
        send_msg_to_bot(&Client::new(), bot, &msg).await?;
    }
    Ok(())
}

pub fn get_yesterday() -> String {
    let now = chrono::Local::now().sub(chrono::Duration::days(1));
    now.format("%Y%m%d").to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Config;
    use crate::xxscore::fetcher::MemberScore;
    use async_trait::async_trait;
    use tokio::fs;

    struct MockFetcher {
        data: MemberScore,
    }

    #[async_trait]
    impl Fetcher for MockFetcher {
        async fn get_score(&self, _: &str) -> Result<MemberScore> {
            Ok(self.data.clone())
        }
    }

    #[tokio::test]
    async fn test_cmd() -> Result<()> {
        tracing_subscriber::fmt::init();

        let conf_str = include_str!("../../config.toml");
        let c = toml::from_str::<Config>(conf_str)?;

        let j: MemberScore =
            serde_json::from_str(&fs::read_to_string("./src/xxscore/test.json").await?)?;

        let xx_fetcher = MockFetcher { data: j };

        daily_score(
            "20230606",
            &xx_fetcher,
            c.notice_bot.iter().map(|s| s.as_str()).collect(),
            c.org_id,
        )
        .await
    }
}

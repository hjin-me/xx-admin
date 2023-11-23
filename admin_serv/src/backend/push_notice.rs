use anyhow::Result;
use tracing::instrument;
use wx::{MsgApi, MP};

#[instrument(skip(mp))]
pub async fn push_notice(
    mp: &MP,
    notice_id: Option<Vec<String>>,
    notice_bot: Option<Vec<String>>,
    notice_text: Option<String>,
) -> Result<()> {
    let default_emoticon = include_bytes!("./notice.png");
    if let Some(uids) = notice_id {
        let to_user = uids.join("|");
        mp.send_image_msg(&to_user, default_emoticon).await?;
        if let Some(text) = notice_text.clone() {
            mp.send_text_msg(&to_user, &text).await?;
        }
    }
    if let Some(bots) = notice_bot {
        for bot_api in bots {
            mp.send_bot_image(default_emoticon, &bot_api).await?;
            if let Some(text) = notice_text.clone() {
                mp.send_bot_text(&text, &bot_api).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::backend::config::AdminConfig;
    use super::*;

    #[tokio::test]
    async fn test_push_notice() -> Result<()> {
        let p: AdminConfig = toml::from_str(include_str!("../../config.toml"))?;

        let mp = wx::MP::new(&p.corp_id, &p.corp_secret, p.agent_id);

        for x in p.notice_schedule {
            push_notice(
                &mp,
                x.notice_id,
                x.notice_bot,
                None,
                // Some("学不动了，来点士力架吧".to_string()),
            )
            .await?;
        }

        Ok(())
    }
}

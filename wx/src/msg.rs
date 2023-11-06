use anyhow::Result;
#[async_trait::async_trait]
pub trait MsgApi {
    async fn recall_msgs(&self, msgs: Vec<String>) -> Result<()>;
    async fn send_image_msg(&self, to_user: &str, img_data: &[u8]) -> Result<String>;
    async fn send_text_msg(&self, to_user: &str, msg: &str) -> Result<String>;
    async fn send_markdown_msg(&self, to_user: &str, msg: &str) -> Result<String>;
}

use crate::MP;
use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

#[async_trait::async_trait]
impl MsgApi for MP {
    async fn recall_msgs(&self, msgs: Vec<String>) -> Result<()> {
        let token = self.get_token().await?;
        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/recall?access_token={}",
            token
        );
        for msg_id in msgs {
            let res = self
                .client
                .post(&api)
                .json(&json!({ "msgid": msg_id }))
                .send()
                .await?;
            let resp_status = res.status();
            if !resp_status.is_success() {
                return Err(anyhow!(
                    "撤回消息失败 error: [{}] {}",
                    resp_status,
                    res.text().await?
                ));
            }
        }

        Ok(())
    }

    async fn send_image_msg(&self, to_user: &str, img_data: &[u8]) -> Result<String> {
        let token = self.get_token().await?;
        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/media/upload?access_token={}",
            token
        );
        let part = reqwest::multipart::Part::bytes(img_data.to_vec())
            .file_name("qr.png")
            .mime_str("image/png")?;
        let resp = self
            .client
            .post(api)
            .query(&[("type", "image")])
            .multipart(reqwest::multipart::Form::new().part("media", part))
            .send()
            .await?;
        let resp_status = resp.status();
        let data_raw = resp.text().await?;

        let data = serde_json::from_str::<UploadMediaResponse>(&data_raw).map_err(|e| {
            anyhow!(
                "send_image_msg failed, {:?}, text: [{}]{}",
                e,
                resp_status,
                data_raw
            )
        })?;

        info!("上传图片， [{}]{:?}", resp_status, &data);

        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        let resp = self
            .client
            .post(api)
            .json(&json!({
                "touser": to_user,
                "msgtype": "image",
                "agentid": self.agent_id,
                "image": {
                    "media_id": data.media_id
                }
            }))
            .send()
            .await?;

        let resp_status = resp.status();
        let data_raw = resp.text().await?;

        let data = serde_json::from_str::<BasicResponse>(&data_raw).map_err(|e| {
            anyhow!(
                "send_image_msg failed, {:?}, text: [{}]{}",
                e,
                resp_status,
                data_raw
            )
        })?;

        info!("发送图片消息，[{}]{:?}", resp_status, &data);
        Ok(data.msg_id)
    }

    async fn send_text_msg(&self, to_user: &str, msg: &str) -> Result<String> {
        let token = self.get_token().await?;
        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        let resp = self
            .client
            .post(api)
            .json(&json!({
                "agentid": self.agent_id,
                "touser": to_user,
                "msgtype": "text",
                "text": {
                    "content": msg
                }
            }))
            .send()
            .await?;
        let resp_status = resp.status();
        let data_raw = resp.text().await?;

        let data = serde_json::from_str::<BasicResponse>(&data_raw).map_err(|e| {
            anyhow!(
                "send_image_msg failed, {:?}, text: [{}]{}",
                e,
                resp_status,
                data_raw
            )
        })?;
        info!("发送文本消息, [{}]{:?}", resp_status, data);
        Ok(data.msg_id)
    }
    async fn send_markdown_msg(&self, to_user: &str, msg: &str) -> Result<String> {
        let token = self.get_token().await?;
        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        let resp = self
            .client
            .post(api)
            .json(&json!({
                "agentid": self.agent_id,
                "touser": to_user,
                "msgtype": "markdown",
                "markdown": {
                    "content": msg
                }
            }))
            .send()
            .await?;

        let resp_status = resp.status();
        let data_raw = resp.text().await?;

        let data = serde_json::from_str::<BasicResponse>(&data_raw).map_err(|e| {
            anyhow!(
                "send_image_msg failed, {:?}, text: [{}]{}",
                e,
                resp_status,
                data_raw
            )
        })?;
        info!("发送 Markdown 消息, [{}]{:?}", resp_status, data);
        Ok(data.msg_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct UploadMediaResponse {
    // #[serde(rename = "errcode")]
    // err_code: i64,
    // #[serde(rename = "errmsg")]
    // err_msg: String,
    #[serde(default)]
    media_id: String,
    // #[serde(default)]
    // created_at: String,
    // #[serde(default, rename = "type")]
    // media_type: String,
}
#[derive(Debug, Clone, Deserialize)]
struct BasicResponse {
    // #[serde(rename = "errcode")]
    // err_code: i64,
    // #[serde(rename = "errmsg")]
    // err_msg: String,
    #[serde(rename = "msgid")]
    msg_id: String,
}

//curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key='
// -H 'Content-Type: application/json'
// -d "{\"msgtype\":\"text\",\"text\":{\"content\":\"$NOTICE_MSG\"}}"
pub async fn send_msg_to_bot(c: &Client, api: &str, msg: &str) -> Result<()> {
    let resp = c
        .post(api)
        .json(&json!({
            "msgtype": "markdown",
            "markdown": {
                "content": msg
            }
        }))
        .send()
        .await?;
    info!(
        "企业微信机器人返回 bot resp: [{}]{:?}",
        resp.status(),
        resp.text().await?
    );
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Deserialize)]
    struct Conf {
        corp_id: String,
        corp_secret: String,
        agent_id: i64,
        to_user: String,
    }
    #[tokio::test]
    async fn test_mp() -> Result<()> {
        let conf: Conf = serde_json::from_str(include_str!("../config.json"))?;
        let b = include_bytes!("./test_upload.png");
        let mp = MP::new(&conf.corp_id, &conf.corp_secret, conf.agent_id);
        mp.send_image_msg(&conf.to_user, b).await?;
        mp.send_text_msg(&conf.to_user, "hello world").await?;
        Ok(())
    }
}

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
struct UploadMediaResponse {
    #[serde(rename = "errcode")]
    err_code: i64,
    #[serde(rename = "errmsg")]
    err_msg: String,
    #[serde(default)]
    media_id: String,
    // #[serde(default)]
    // created_at: String,
    // #[serde(default, rename = "type")]
    // media_type: String,
}
#[derive(Debug, Clone, Deserialize)]
struct BasicResponse {
    #[serde(rename = "errcode")]
    err_code: i64,
    #[serde(rename = "errmsg")]
    err_msg: String,
    #[serde(rename = "msgid")]
    msg_id: String,
}

pub async fn send_image_msg(
    client: &Client,
    to_user: &str,
    img_data: &[u8],
    wechat_proxy: &str,
) -> Result<String> {
    let media_api = format!("{}/cgi-bin/media/upload", wechat_proxy);
    let part = reqwest::multipart::Part::bytes(img_data.to_vec())
        .file_name("qr.png")
        .mime_str("image/png")?;
    let resp = client
        .post(media_api)
        .query(&[("type", "image")])
        .multipart(reqwest::multipart::Form::new().part("media", part))
        .send()
        .await?;
    // let resp_status = resp.status();
    let data_raw = resp.text().await?;

    let data = serde_json::from_str::<UploadMediaResponse>(&data_raw)
        .map_err(|e| anyhow!("send_image_msg failed, {:?}, text: {}", e, data_raw))?;

    // println!("status: {}, resp: {}", resp_status, data_raw);

    let api = format!("{}/cgi-bin/message/send", wechat_proxy);
    let resp = client
        .post(api)
        .json(&json!({
            "touser": to_user,
            "msgtype": "image",
            "image": {
                "media_id": data.media_id
            }
        }))
        .send()
        .await?;

    // let resp_status = resp.status();
    let data_raw = resp.text().await?;

    let data = serde_json::from_str::<BasicResponse>(&data_raw)
        .map_err(|e| anyhow!("send_image_msg failed, {:?}, text: {}", e, data_raw))?;
    Ok(data.msg_id)
}

pub async fn send_text_msg(
    client: &Client,
    to_user: &str,
    msg: &str,
    wechat_proxy: &str,
) -> Result<String> {
    let api = format!("{}/cgi-bin/message/send", wechat_proxy);
    let resp = client
        .post(api)
        .json(&json!({
            "touser": to_user,
            "msgtype": "text",
            "text": {
                "content": msg
            }
        }))
        .send()
        .await?;
    // let resp_status = resp.status();
    let data_raw = resp.text().await?;

    let data = serde_json::from_str::<BasicResponse>(&data_raw)
        .map_err(|e| anyhow!("send_image_msg failed, {:?}, text: {}", e, data_raw))?;
    Ok(data.msg_id)
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
    info!("bot resp: {:?}", resp.text().await?);
    Ok(())
}

use anyhow::Result;
use std::fmt;
use std::fmt::Display;
#[async_trait::async_trait]
pub trait MsgApi {
    async fn recall_msgs(&self, msgs: Vec<String>) -> Result<()>;
    async fn send_image_msg(&self, to_user: &str, img_data: &[u8]) -> Result<String>;
    async fn send_text_msg(&self, to_user: &str, msg: &str) -> Result<String>;
    async fn send_markdown_msg(&self, to_user: &str, msg: &str) -> Result<String>;
    async fn send_bot_msg(&self, msg: &str, api: &str) -> Result<()>;
    async fn send_msg(&self, d: SendMsgReq) -> Result<String>;
}

use crate::MP;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
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

        self.send_msg(SendMsgReq::Image(SendImageMsgReq {
            common: SendMsgCommon {
                to_user: Some(to_user.to_string()),
                msg_type: MsgType::Image,
                agent_id: self.agent_id,
                ..Default::default()
            },
            image: MediaContent {
                media_id: data.media_id,
                ..Default::default()
            },
        }))
        .await
    }

    async fn send_text_msg(&self, to_user: &str, msg: &str) -> Result<String> {
        self.send_msg(SendMsgReq::Text(SendTextMsgReq {
            common: SendMsgCommon {
                to_user: Some(to_user.to_string()),
                msg_type: MsgType::Text,
                agent_id: self.agent_id,
                ..Default::default()
            },
            text: TextContent {
                content: msg.to_string(),
            },
        }))
        .await
    }

    async fn send_markdown_msg(&self, to_user: &str, msg: &str) -> Result<String> {
        self.send_msg(SendMsgReq::Markdown(SendMarkdownMsgReq {
            common: SendMsgCommon {
                to_user: Some(to_user.to_string()),
                msg_type: MsgType::Markdown,
                agent_id: self.agent_id,
                ..Default::default()
            },
            markdown: TextContent {
                content: msg.to_string(),
            },
        }))
        .await
    }

    //curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key='
    // -H 'Content-Type: application/json'
    // -d "{\"msgtype\":\"text\",\"text\":{\"content\":\"$NOTICE_MSG\"}}"
    async fn send_bot_msg(&self, msg: &str, api: &str) -> Result<()> {
        let resp = self
            .client
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

    async fn send_msg(&self, mut d: SendMsgReq) -> Result<String> {
        let token = self.get_token().await?;
        let api = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        d.set_agent_id(self.agent_id);

        let resp = self.client.post(api).json(&d).send().await?;

        let resp_status = resp.status();
        let data_raw = resp.text().await?;

        let data = serde_json::from_str::<BasicResponse>(&data_raw).map_err(|e| {
            anyhow!(
                "send_msg failed, {:?}, text: [{}]{}",
                e,
                resp_status,
                data_raw
            )
        })?;
        info!("发送消息, [{}]{:?}", resp_status, data);
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

#[derive(Debug, Clone)]
enum MsgType {
    Text,
    Image,
    Voice,
    Video,
    File,
    TextCard,
    News,
    Mpnews,
    Markdown,
    // MiniprogramNotice,
    // Taskcard,
    // InteractiveTaskcard,
    // TemplateCard,
}
impl Default for MsgType {
    fn default() -> Self {
        Self::Text
    }
}
impl Display for MsgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MsgType::Text => write!(f, "text"),
            MsgType::Image => write!(f, "image"),
            MsgType::Voice => write!(f, "voice"),
            MsgType::Video => write!(f, "video"),
            MsgType::File => write!(f, "file"),
            MsgType::Markdown => write!(f, "markdown"),
            MsgType::TextCard => write!(f, "textcard"),
            MsgType::News => write!(f, "news"),
            MsgType::Mpnews => write!(f, "mpnews"),
        }
    }
}

impl From<String> for MsgType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => MsgType::Text,
            "image" => MsgType::Image,
            "voice" => MsgType::Voice,
            "video" => MsgType::Video,
            "file" => MsgType::File,
            "markdown" => MsgType::Markdown,
            "textcard" => MsgType::TextCard,
            "news" => MsgType::News,
            "mpnews" => MsgType::Mpnews,
            _ => MsgType::Text,
        }
    }
}
impl MsgType {
    fn as_str(&self) -> &'static str {
        match self {
            MsgType::Text => "text",
            MsgType::Image => "image",
            MsgType::Voice => "voice",
            MsgType::Video => "video",
            MsgType::File => "file",
            MsgType::Markdown => "markdown",
            MsgType::TextCard => "textcard",
            MsgType::News => "news",
            MsgType::Mpnews => "mpnews",
        }
    }
}

impl<'de> Deserialize<'de> for MsgType {
    fn deserialize<D>(deserializer: D) -> Result<MsgType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(MsgType::from(s))
    }
}
impl Serialize for MsgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct TextContent {
    content: String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
struct MediaContent {
    media_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
struct TextCardContent {
    title: String,
    description: String,
    url: String,
    #[serde(rename = "btntxt")]
    btn_txt: String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
struct NewsContent {
    articles: Vec<NewsArticle>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
struct NewsArticle {
    title: String,
    description: String,
    url: String,
    #[serde(rename = "picurl")]
    pic_url: String,
}
// #[derive(Serialize, Deserialize, Debug)]
// struct MpnewsContent {
//     articles: Vec<MpArticle>,
// }
// #[derive(Serialize, Deserialize, Debug)]
// struct MpArticle {
//     title: String,
//     thumb_media_id: String,
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum SendMsgReq {
    Text(SendTextMsgReq),
    Image(SendImageMsgReq),
    Voice(SendVoiceMsgReq),
    Video(SendVideoMsgReq),
    File(SendFileMsgReq),
    Markdown(SendMarkdownMsgReq),
    TextCard(SendTextCardMsgReq),
    News(SendNewsMsgReq),
    // Mpnews(SendMpnewsMsgReq),
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SendMsgCommon {
    #[serde(rename = "touser", skip_serializing_if = "Option::is_none")]
    pub to_user: Option<String>,
    #[serde(rename = "toparty", skip_serializing_if = "Option::is_none")]
    pub to_party: Option<String>,
    #[serde(rename = "totag", skip_serializing_if = "Option::is_none")]
    pub to_tag: Option<String>,
    #[serde(rename = "msgtype")]
    pub msg_type: MsgType,
    #[serde(rename = "agentid", default)]
    pub agent_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_id_trans: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_duplicate_check: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_check_interval: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendImageMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    image: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendTextMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    text: TextContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendVoiceMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    voice: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendVideoMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    video: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendFileMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    file: MediaContent,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendMarkdownMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    markdown: TextContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendTextCardMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    textcard: TextCardContent,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SendNewsMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    news: NewsContent,
}
// #[derive(Serialize, Deserialize, Debug)]
// pub struct SendMpnewsMsgReq {
//     #[serde(flatten)]
//     common: SendMsgCommon,
//     mpnews: ,
// }

impl SendMsgReq {
    fn set_agent_id(&mut self, agent_id: i64) {
        match self {
            SendMsgReq::Text(d) => d.common.agent_id = agent_id,
            SendMsgReq::Image(d) => d.common.agent_id = agent_id,
            SendMsgReq::Voice(d) => d.common.agent_id = agent_id,
            SendMsgReq::Video(d) => d.common.agent_id = agent_id,
            SendMsgReq::File(d) => d.common.agent_id = agent_id,
            SendMsgReq::Markdown(d) => d.common.agent_id = agent_id,
            SendMsgReq::TextCard(d) => d.common.agent_id = agent_id,
            SendMsgReq::News(d) => d.common.agent_id = agent_id,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_json_diff::assert_json_eq;

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

    #[test]
    fn test_json() {
        dbg!(serde_json::from_str::<MsgType>("\"image\"").unwrap());
        dbg!(serde_json::to_string(&MsgType::Image).unwrap());
        let cases = vec![
            (
                r#"{ "touser": "abc", "msgtype": "text", "text": { "content": "content" }}"#,
                r#"{"touser":"abc","msgtype":"text","agentid":0,"text":{"content":"content"}}"#,
            ),
            (
                r#"{ "touser": "abc", "msgtype" : "image", "image" : { "media_id" : "MEDIA_ID" }}"#,
                r#"{"touser":"abc","msgtype":"image","agentid":0,"image":{"media_id":"MEDIA_ID"}}"#,
            ),
            (
                r#"{
  "touser": "UserID1|UserID3",
  "toparty": "PartyID1|PartyID2",
  "totag": "TagID1 | TagID2",
  "msgtype": "voice",
  "agentid": 3,
  "voice": {
    "media_id": "MEDIA_ID"
  },
  "enable_duplicate_check": 0,
  "duplicate_check_interval": 1800
}"#,
                r#"{"touser":"UserID1|UserID3","toparty":"PartyID1|PartyID2","totag":"TagID1 | TagID2","msgtype":"voice","agentid":3,"enable_duplicate_check":0,"duplicate_check_interval":1800,"voice":{"media_id":"MEDIA_ID"}}"#,
            ),
            (
                r#"{
   "touser" : "UserIID3",
   "toparty" : "ParrtyID2",
   "totag" : "TaID2",
   "msgtype" : "video",
   "agentid" : 1,
   "video" : {
        "media_id" : "MEDIA_ID",
        "title" : "Title",
       "description" : "Description"
   },
   "safe":0,
   "enable_duplicate_check": 0,
   "duplicate_check_interval": 1800
}"#,
                r#"{
  "touser": "UserIID3",
  "toparty": "ParrtyID2",
  "totag": "TaID2",
  "msgtype": "video",
  "agentid": 1,
  "safe": 0,
  "enable_duplicate_check": 0,
  "duplicate_check_interval": 1800,
  "video": {
    "media_id": "MEDIA_ID",
    "title": "Title",
    "description": "Description"
  }
}"#,
            ),
            (
                r#"{
   "touser" : "UserID1",
   "toparty" : "PartyID1|",
   "totag" : "TagID1 | TagID2",
   "msgtype" : "file",
   "agentid" : 1,
   "file" : {
        "media_id" : "1Yv-zXfHjSjU-7LH-GwtYqDGS-zz6w22KmWAT5COgP7o"
   },
   "safe":0,
   "enable_duplicate_check": 0,
   "duplicate_check_interval": 1800
}"#,
                r#"{"touser":"UserID1","toparty":"PartyID1|","totag":"TagID1 | TagID2","msgtype":"file","agentid":1,"safe":0,"enable_duplicate_check":0,"duplicate_check_interval":1800,"file":{"media_id":"1Yv-zXfHjSjU-7LH-GwtYqDGS-zz6w22KmWAT5COgP7o"}}"#,
            ),
        ];
        for x in cases {
            let t = serde_json::from_str::<SendMsgReq>(x.0).unwrap();
            let s = serde_json::to_string(&t).unwrap();
            let vl = serde_json::from_str::<serde_json::Value>(&s).unwrap();
            let vr = serde_json::from_str::<serde_json::Value>(x.1).unwrap();
            assert_json_eq!(vl, vr);
        }

        let mut t = SendMsgReq::Text(SendTextMsgReq {
            common: SendMsgCommon {
                to_user: None,
                to_party: None,
                to_tag: None,
                msg_type: MsgType::Text,
                agent_id: 0,
                safe: None,
                enable_id_trans: None,
                enable_duplicate_check: None,
                duplicate_check_interval: None,
            },
            text: TextContent {
                content: "".to_string(),
            },
        });
        t.set_agent_id(666);
        dbg!(serde_json::to_string(&t).unwrap());
    }
}

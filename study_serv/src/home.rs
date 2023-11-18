use crate::qr::gen_qr_data_uri;
use crate::wx_redirect::*;
use anyhow::Result;
use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use gloo::timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use study::{State, Ticket};
use tracing::info;

pub fn app(cx: Scope) -> Element {
    use futures_util::stream::StreamExt;
    let s_id = use_state(cx, || 0u64);
    let err_msg = use_state(cx, || "".to_string());
    let session_state = use_state(cx, || State::Prepare);

    let tx = use_coroutine(cx, |mut rx: UnboundedReceiver<u64>| {
        to_owned![err_msg, session_state];
        async move {
            while let Some(id) = rx.next().await {
                loop {
                    let state = get_state(id).await;
                    match state {
                        Ok(s) => {
                            info!("state is {:?}", s);
                            session_state.set(s.clone());
                            err_msg.set("自动查询更新状态中...".to_string());
                            if let State::Complete(_) = s {
                                break;
                            }
                        }
                        Err(e) => {
                            err_msg.set(e.to_string());
                        }
                    }
                    TimeoutFuture::new(2000).await;
                }
            }
        }
    });

    let ui = match session_state.get().clone() {
        State::Prepare => {
            rsx! {
                h1 { "1. 点击按钮" }
                p {
                    button { onclick: move |_| {
                            to_owned![s_id, err_msg, tx];
                            async move {
                                let data = match create_task().await {
                                    Ok(data) => {
                                        s_id.set(data);
                                        data
                                    }
                                    Err(e) => {
                                        err_msg.set(e.to_string());
                                        return;
                                    }
                                };
                                tx.send(data);
                            }
                        },
                        "请让我学习"
                    }
                }
            }
        }
        State::Init => {
            rsx! { p { "正在启动浏览器，稍等片刻..." } }
        }
        State::WaitingLogin(Ticket { ticket }) => match ticket_conv(&ticket) {
            Ok(d) => {
                let u = format!("dtxuexi://appclient/page/study_feeds?url={}", d.0);
                let img = d.1;
                rsx! {
                    h1 { "2. 点击下方登录：" }
                    pre { a { id: "login", target: "_blank", rel: "noopener", href: "{u}", "{u}" } }
                    h3 { "点击上方链接登录，或使用学习强国扫描下方二维码" }
                    img { src: "{img}" }
                }
            }
            Err(e) => {
                rsx! { p { "{e:?}" } }
            }
        },
        State::Logged(nick_name) => {
            let nick_name = nick_name.clone();
            rsx! { p { "Hi {nick_name}, 你已经扫码登陆，即将开始学习。" } }
        }
        State::Broken(e) => {
            let err_msg = e.clone();
            rsx! {
                p { "发生了奇怪的错误：{err_msg}" }
                p { "稍后可以刷新页面再试试。" }
            }
        }
        State::Complete((nick_name, today_score)) => {
            rsx! { p { "{nick_name}，今日学习的分数是 {today_score}" } }
        }
        State::Ready => {
            rsx! { p { "即将开始学习" } }
        }
    };

    cx.render(rsx! {
        WxWorkRedirect {}

        ui,

        p {
            "中国要强盛、要复兴，就一定要大力发展科学技术，努力成为世界主要科学中心和创新高地。我们比历史上任何时期都更接近中华民族伟大复兴的目标，我们比历史上任何时期都更需要建设世界科技强国！"
        }

        pre { "当前状态：{session_state:?}\n\n", "{err_msg}" }
    })
}

// #[server]
// async fn post_server_data(data: String) -> Result<(), ServerFnError> {
//     let axum::extract::Host(host): axum::extract::Host = extract().await?;
//     println!("Server received: {}", data);
//     println!("{:?}", host);
//
//     Ok(())
// }

#[server(GetTicket, "/xx/api")]
async fn get_ticket(s_id: u64) -> Result<String, ServerFnError> {
    match crate::xx::try_get_ticket(s_id).await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

#[server(CreateTask, "/xx/api")]
async fn create_task() -> Result<u64, ServerFnError> {
    match crate::xx::start_new_task().await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

#[server(GetUsername, "/xx/api")]
async fn get_username(s_id: u64) -> Result<String, ServerFnError> {
    match crate::xx::try_get_current_user(s_id).await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

#[server(GetState, "/xx/api")]
async fn get_state(s_id: u64) -> Result<State, ServerFnError> {
    match crate::xx::try_get_state(s_id).await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

fn ticket_conv(s: &str) -> Result<(String, String)> {
    let data_uri = gen_qr_data_uri(&s)?;
    let mut ticket = "".to_string();
    ticket.extend(form_urlencoded::byte_serialize(s.as_bytes()));
    Ok((ticket, data_uri))
}

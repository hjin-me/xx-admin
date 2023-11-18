use crate::sleep::sleep;
use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use study::State;
use tracing::info;

pub fn app(cx: Scope) -> Element {
    let text = use_state(cx, || "".to_string());
    let s_id = use_state(cx, || 0u64);
    let err_msg = use_state(cx, || "".to_string());
    let session_state = use_state(cx, || State::Init);

    let tx = use_coroutine(cx, |_| {
        to_owned![s_id, err_msg, session_state];
        async move {
            loop {
                let id = s_id.get().clone();
                let state = get_state(id).await;
                match state {
                    Ok(s) => {
                        info!("state is {:?}", s);
                        session_state.set(s);
                        err_msg.set("".to_string());
                    }
                    Err(e) => {
                        err_msg.set(e.to_string());
                    }
                }
            }
        }
    });
    let refresh_btn = rsx! {
        p {
            button { onclick: move |_| {
                    to_owned![s_id, text];
                    async move {
                        if let Ok(data) = get_ticket(s_id.get().clone()).await {
                            info!("Client received: {}", data);
                            text.set(
                                format!("dtxuexi://appclient/page/study_feeds?url={}", data.clone()),
                            );
                        }
                    }
                },
                "获取登陆链接"
            }
        }
    };

    cx.render(rsx! {
        div { "企业微信点击右上角，在内置浏览器打开" }
        p { "{err_msg}" }
        h1 { "1. 点击按钮" }
        p {
            button { onclick: move |_| {
                    to_owned![s_id, text, err_msg, tx];
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
        if text.get() != "" {
            rsx! {
                h1 { "2. 点击下方登录：" }
                pre { a { id: "login", target: "_blank", rel: "noopener", href: "{text}", "{text}" } }
                h3 { "点击上方链接登录，没有显示就出了问题" }
            }
        } else {
            rsx! {
                ""
            }
        }

        p {
            "中国要强盛、要复兴，就一定要大力发展科学技术，努力成为世界主要科学中心和创新高地。我们比历史上任何时期都更接近中华民族伟大复兴的目标，我们比历史上任何时期都更需要建设世界科技强国！"
        }

        if *s_id.get() != 0 {
            rsx! {refresh_btn}
        } else {
            rsx! {""}
        }
        p { "当前用户：{session_state:?}" }

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
    tokio::time::sleep(Duration::from_secs(1)).await;
    match crate::xx::try_get_state(s_id).await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

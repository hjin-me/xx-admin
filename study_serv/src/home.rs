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
use study_core::{State, Ticket};
use tracing::info;

pub fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Layout { Study {} }
    })
}
#[derive(Props)]
struct LayoutProps<'a> {
    children: Element<'a>,
}
fn Layout<'a>(cx: Scope<'a, LayoutProps<'a>>) -> Element {
    cx.render(rsx! {
        div { class: "relative bg-white dark:bg-dark",
            div { class: "container mx-auto mt-8",
                div { class: "flex flex-wrap items-center mx-4",
                    div { class: "w-full px-4 lg:w-5/12", &cx.props.children }
                }
            }
        }
    })
}

pub fn Study(cx: Scope) -> Element {
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
                    button {
                        class: "middle none center mr-4 rounded-lg bg-blue-500 py-3 px-6 font-sans text-xs font-bold uppercase text-white shadow-md shadow-blue-500/20 transition-all hover:shadow-lg hover:shadow-blue-500/40 focus:opacity-[0.85] focus:shadow-none active:opacity-[0.85] active:shadow-none disabled:pointer-events-none disabled:opacity-50 disabled:shadow-none",
                        r#type: "button",
                        onclick: move |_| {
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
        State::WaitingLogin((ticket, _expired_at)) => match ticket_conv(&ticket) {
            Ok(d) => {
                let u = format!("dtxuexi://appclient/page/study_feeds?url={}", d.0);
                let img = d.1;
                rsx! {
                    h1 { "2. 点击下方登录：" }
                    a {
                        class: "text-blue-500 underline hover:text-blue-700 whitespace-normal break-all",
                        target: "_blank",
                        rel: "noopener",
                        href: "{u}",
                        "{u}"
                    }
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
            rsx! {
                p { class: "block font-sans text-xl font-normal leading-relaxed text-inherit antialiased",
                    "Hi {nick_name}："
                }
                p { "你已经扫码登陆，学习即将开始，可以关闭浏览器了。" }
            }
        }
        State::Broken(e) => {
            let err_msg = e.clone();
            rsx! {
                p { "发生了奇怪的错误：{err_msg}" }
                p { "稍后可以刷新页面再试试。" }
            }
        }
        State::Complete((nick_name, today_score)) => {
            rsx! {
                p { class: "block font-sans text-xl font-normal leading-relaxed text-inherit antialiased",
                    "Hi {nick_name}："
                }
                p { "今日学习的分数是 {today_score}" }
            }
        }
        State::Ready => {
            rsx! { p { "即将开始学习" } }
        }
        State::Learning((nick_name, logs)) => {
            let log_records = logs.into_iter().map(|(key, c, t)| {
                // to_owned![key, c, t];
                rsx!{
                    p { class: "block font-sans text-xl font-normal leading-relaxed text-inherit antialiased",
                        "{key}：{c}/{t}"
                    }
                }
            });
            rsx! {
                p { class: "block font-sans text-xl font-normal leading-relaxed text-inherit antialiased",
                    "Hi {nick_name}："
                }
                p { "学习中..." }
                log_records
            }
        }
    };

    cx.render(rsx! {
        WxWorkRedirect {}

        div { class: "relative w-full flex flex-col text-gray-700 bg-white shadow-md w-96 rounded-xl bg-clip-border",
            div { class: "p-6", ui }
        }

        p { class: "my-8 block font-sans text-base font-light leading-relaxed text-inherit antialiased",
            "中国要强盛、要复兴，就一定要大力发展科学技术，努力成为世界主要科学中心和创新高地。我们比历史上任何时期都更接近中华民族伟大复兴的目标，我们比历史上任何时期都更需要建设世界科技强国！"
        }

        div { class: "peer h-full w-full resize-none rounded-[7px] border border-blue-gray-200 border-t-transparent bg-transparent px-3 py-2.5 font-sans text-sm font-normal text-blue-gray-700 outline outline-0 transition-all placeholder-shown:border placeholder-shown:border-blue-gray-200 placeholder-shown:border-t-blue-gray-200 focus:border-2 focus:border-pink-500 focus:border-t-transparent focus:outline-0 disabled:resize-none disabled:border-0 disabled:bg-blue-gray-50",
            "当前状态：{session_state:?}\n\n"
            "{err_msg}"
        }
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

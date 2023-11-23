use crate::qr::gen_qr_data_uri;
use crate::state::State;
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use gloo::timers::future::TimeoutFuture;
use tracing::info;
use futures_util::stream::StreamExt;

pub fn app(cx: Scope) -> Element {
    let st = use_state(cx, || State::Prepare);

    let tx = use_coroutine(cx, |mut rx: UnboundedReceiver<()>| {
        to_owned![st];
        async move {
            while let Some(_) = rx.next().await {
                loop {
                    let state = get_state().await;
                    match state {
                        Ok(s) => {
                            info!("state is {:?}", s);
                            st.set(s.clone());
                            if let State::Complete(_) = s {
                                break;
                            }
                        }
                        Err(e) => {
                            st.set(State::Broken(e.to_string()));
                            break;
                        }
                    }
                    TimeoutFuture::new(1000).await;
                }
            }
        }
    });
    tx.send(());
    let ui = match st.get().clone() {
        State::Prepare => {
            rsx! { p { "正在准备" } }
        }
        State::Init => {
            rsx! { p { "正在启动浏览器，稍等片刻..." } }
        }
        State::WaitingLogin((ticket, _expired_at)) => match gen_qr_data_uri(&ticket) {
            Ok(img) => {
                rsx! {
                    h1 { "学习强国扫码登陆" }
                    img { src: "{img}" }
                }
            }
            Err(e) => {
                rsx! { p { "{e:?}" } }
            }
        },
        State::Broken(e) => {
            let err_msg = e.clone();
            rsx! {
                p { "发生了奇怪的错误：{err_msg}" }
                p { "稍后可以刷新页面再试试。" }
            }
        }
        State::Complete(_) => {
            rsx! { p { "学习强国分数统计完成" } }
        }
        State::Ready => {
            rsx! { p { "即将开始学习" } }
        }
        State::Logged => {
            rsx! { p { "登陆了" } }
        }
    };

    cx.render(rsx! {
        h1 { "你好世界" }
        ui
    })
}

#[server]
async fn get_state() -> Result<State, ServerFnError> {
    match crate::backend::api::try_get_state().await {
        Ok(s) => Ok(s),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

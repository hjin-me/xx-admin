//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features ssr --release
//! ```

#![allow(non_snake_case, unused)]

#[cfg(feature = "ssr")]
mod backend;
mod state;
#[cfg(feature = "ssr")]
mod xx;

use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, trace};

fn app(cx: Scope) -> Element {
    let text = use_state(cx, || "".to_string());
    let s_id = use_state(cx, || 0u64);
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
        h1 { "1. 点击按钮" }
        p {
            button { onclick: move |_| {
                    to_owned![s_id, text];
                    async move {
                        if let Ok(data) = create_task().await {
                            info!("Client received: {}", data);
                            s_id.set(data);
                            if let Ok(data) = get_ticket(data).await {
                                text.set(
                                    format!("dtxuexi://appclient/page/study_feeds?url={}", data.clone()),
                                );
                            }
                        }
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
    match xx::try_get_ticket(s_id).await {
        Ok(s) => Ok(s),
        Err(e) => Ok(format!("{}", e)),
    }
}

#[server(CreateTask, "/xx/api")]
async fn create_task() -> Result<u64, ServerFnError> {
    match xx::start_new_task().await {
        Ok(s) => Ok(s),
        Err(e) => Err(dioxus_fullstack::prelude::ServerFnError::ServerError(
            e.to_string(),
        )),
    }
}

fn main() {
    #[cfg(feature = "web")]
    {
        tracing_wasm::set_as_global_default();
        LaunchBuilder::new_with_props(app, ()).launch();
    }

    #[cfg(feature = "ssr")]
    {
        use axum::routing::*;
        use axum::Extension;
        use study::{bb8, StateSession, XxManager};
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let _g = crate::backend::otel::init_tracing_subscriber("study");
                trace!("Starting up");
                let manager = XxManager::new();
                trace!("init browsers");
                let pool = bb8::Pool::builder()
                    .max_size(2)
                    .min_idle(Some(1))
                    .idle_timeout(Some(Duration::from_secs(170)))
                    // .connection_timeout(std::time::Duration::from_secs(30))
                    .build(manager)
                    .await
                    .unwrap();

                trace!("init sessions");
                let ss = StateSession::new(&pool);

                // build our application with some routes
                let app = Router::new()
                    // Server side render the application, serve static assets, and register server functions
                    .serve_dioxus_application("/xx/api", ServeConfigBuilder::new(app, ()))
                    .layer(Extension(pool))
                    .layer(Extension(ss));

                // run it
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));

                info!("http://127.0.0.1:3000");
                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });
    }
}

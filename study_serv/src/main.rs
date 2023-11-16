//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features ssr --release
//! ```

#![allow(non_snake_case, unused)]

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

fn app(cx: Scope) -> Element {
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        div {
            "企业微信点击右上角，在内置浏览器打开"
        }
        h2 { "点击登录：" }
        pre {
            a {id:"login",target:"_blank",rel:"noopener", href:"{text}", "{text}"}
            }
        h3 {
            "点击上方链接登录，没有显示就出了问题"
        }
        p {
            "中国要强盛、要复兴，就一定要大力发展科学技术，努力成为世界主要科学中心和创新高地。我们比历史上任何时期都更接近中华民族伟大复兴的目标，我们比历史上任何时期都更需要建设世界科技强国！"
        }
        button {
            onclick: move |_| {
                to_owned![text];
                async move {
                    if let Ok(data) = get_server_data().await {
                        println!("Client received: {}", data);
                        text.set(format!("dtxuexi://appclient/page/study_feeds?url={}",data.clone()));
                        post_server_data(data).await.unwrap();
                    }
                }
            },
            "获取登陆链接"
        }
    })
}

#[server]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    let axum::extract::Host(host): axum::extract::Host = extract().await?;
    println!("Server received: {}", data);
    println!("{:?}", host);

    Ok(())
}

#[server]
async fn get_server_data() -> Result<String, ServerFnError> {
    use axum::Extension;
    use study::{bb8, XxManager};
    let Extension(xx_pool): Extension<bb8::Pool<XxManager>> = extract().await?;
    Ok(match xx::run(xx_pool).await {
        Ok(s) => s,
        Err(e) => format!("{}", e),
    })
    // Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

fn main() {
    #[cfg(feature = "web")]
    {
        tracing_wasm::set_as_global_default();
        LaunchBuilder::new_with_props(app, ()).launch();
    }

    #[cfg(feature = "ssr")]
    {
        tracing_subscriber::fmt::init();
        use axum::routing::*;
        use axum::Extension;
        use study::{bb8, XxManager};
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let manager =
                    XxManager::new("https://techxuexi.js.org/jump/techxuexi-20211023.html?");
                let pool = bb8::Pool::builder()
                    .max_size(2)
                    .min_idle(Some(1))
                    .idle_timeout(Some(Duration::from_secs(170)))
                    // .connection_timeout(std::time::Duration::from_secs(30))
                    .build(manager)
                    .await
                    .unwrap();

                // build our application with some routes
                let app = Router::new()
                    // Server side render the application, serve static assets, and register server functions
                    .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
                    .layer(Extension(pool));
                // .layer(
                //     axum_session_auth::AuthSessionLayer::<
                //         crate::auth::User,
                //         i64,
                //         axum_session_auth::SessionSqlitePool,
                //         sqlx::SqlitePool,
                //     >::new(Some(pool))
                //     .with_config(auth_config),
                // )
                // .layer(axum_session::SessionLayer::new(session_store));

                // run it
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));

                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });
    }
}

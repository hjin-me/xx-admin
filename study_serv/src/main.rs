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

#[derive(Props, PartialEq, Debug, Default, Serialize, Deserialize, Clone)]
struct AppProps {
    count: i32,
}

fn app(cx: Scope<AppProps>) -> Element {
    let state = use_server_future(cx, (), |()| async move { "".to_string() })?.value();

    let mut count = use_state(cx, || 0);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        div {
            "Server state: {state}"
        }
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button {
            onclick: move |_| {
                to_owned![text];
                async move {
                    if let Ok(data) = get_server_data().await {
                        println!("Client received: {}", data);
                        text.set(data.clone());
                        post_server_data(data).await.unwrap();
                    }
                }
            },
            "Run a server function!"
        }
        "Server said: {text}"
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
    Ok(xx::run(xx_pool).await.unwrap())
    // Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

fn main() {
    #[cfg(feature = "web")]
    {
        tracing_wasm::set_as_global_default();
        LaunchBuilder::new_with_props(app, AppProps { count: 0 }).launch();
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
                    // .connection_timeout(std::time::Duration::from_secs(30))
                    .build(manager)
                    .await
                    .unwrap();

                // build our application with some routes
                let app = Router::new()
                    // Server side render the application, serve static assets, and register server functions
                    .serve_dioxus_application(
                        "",
                        ServeConfigBuilder::new(app, AppProps { count: 0 }),
                    )
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

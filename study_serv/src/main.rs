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
    xx::run().await.unwrap();
    Ok("".to_string())
    // Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

fn main() {
    #[cfg(feature = "web")]
    tracing_wasm::set_as_global_default();
    #[cfg(feature = "ssr")]
    tracing_subscriber::fmt::init();

    #[cfg(feature = "web")]
    // Hydrate the application on the client
    dioxus_web::launch_with_props(
        app,
        AppProps { count: 0 },
        dioxus_web::Config::new().hydrate(true),
    );
    #[cfg(feature = "ssr")]
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                tracing::info!("{:?}", tokio::runtime::Handle::current().runtime_flavor());
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            // Server side render the application, serve static assets, and register server functions
                            .serve_dioxus_application(
                                "",
                                ServeConfigBuilder::new(app, AppProps { count: 0 }),
                            )
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
    // LaunchBuilder::new_with_props(app, AppProps { count: 0 }).launch();
}

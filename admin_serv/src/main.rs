// -----------
//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features ssr --release
//! ```

#![allow(non_snake_case, unused)]

#[cfg(feature = "ssr")]
mod backend;
mod home;
mod qr;
pub mod state;

use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use home::app;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

#[cfg(feature = "web")]
fn main() {
    tracing_wasm::set_as_global_default();
    LaunchBuilder::new_with_props(app, ()).launch();
}

#[cfg(any(not(feature = "web"), feature = "ssr"))]
#[tokio::main]
async fn main() {
    use crate::backend::config::AdminConfig;
    use crate::backend::StateSession;
    use axum::routing::*;
    use axum::Extension;
    use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct Args {
        #[arg(short, long, default_value = "./config.toml")]
        config: String,
        #[arg(long)]
        proxy_server: Option<String>,
    }

    let args = Args::parse();

    let _g = infra::otel::init_tracing_subscriber("admin");
    trace!("Starting up, {:?}", args);
    let p: AdminConfig = {
        let contents = std::fs::read_to_string(&args.config).expect("读取配置文件失败");
        toml::from_str(contents.as_str()).expect("解析配置文件失败")
    };
    let mp = wx::MP::new(&p.corp_id, &p.corp_secret, p.agent_id);
    let ss = StateSession::new(
        mp.clone(),
        &p.xx_org_gray_id,
        p.proxy_server.clone(),
        p.notice_bot.clone(),
        p.org_id,
        p.admin_user.clone(),
    )
    .expect("初始化 StateSession 失败");

    let conf_path = args.config;
    tokio::spawn(async move {
        _ = backend::serve(&conf_path).await;
    });

    // build our application with some routes
    let app = Router::new()
        // Server side render the application, serve static assets, and register server functions
        .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
        .layer(Extension(ss))
        .layer(Extension(mp));

    // run it
    let app = app.layer(
        tower::ServiceBuilder::new().layer(tower_http::compression::CompressionLayer::new()),
    );
    let addr = if cfg!(debug_assertions) {
        std::net::SocketAddr::from(([127, 0, 0, 1], 3000))
    } else {
        std::net::SocketAddr::from(([0, 0, 0, 0], 3000))
    };

    info!("listening: http://{}", addr.to_string());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

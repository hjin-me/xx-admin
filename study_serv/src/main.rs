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
mod state;
mod wx_redirect;
#[cfg(feature = "ssr")]
mod xx;

use crate::home::app;
use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, trace};

#[cfg(feature = "web")]
fn main() {
    tracing_wasm::set_as_global_default();
    LaunchBuilder::new_with_props(app, ()).launch();
}

#[cfg(any(not(feature = "web"), feature = "ssr"))]
#[tokio::main]
async fn main() {
    use crate::backend::user_validator::WBList;
    use axum::routing::*;
    use axum::Extension;
    use clap::Parser;
    use study::{bb8, StateSession, XxManager};

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct Args {
        /// Number of times to greet
        #[arg(long, default_value = "10")]
        max_size: u32,
        #[arg(long, default_value = "1")]
        min_idle: u32,
        #[arg(short, long, default_value = "./config.toml")]
        config: String,
    }

    let args = Args::parse();

    let _g = infra::otel::init_tracing_subscriber("study");
    trace!("Starting up, {:?}", args);
    // let config = backend::conf::BaseConf::from_path(&args.config).expect("读取配置文件失败");
    let manager = XxManager::new(WBList::new(&args.config));
    trace!("init browsers");
    let pool = bb8::Pool::builder()
        .max_size(args.max_size)
        .min_idle(Some(args.min_idle))
        .build(manager)
        .await
        .unwrap();

    trace!("init sessions");
    let ss = StateSession::new(pool);

    // build our application with some routes
    let app = Router::new()
        // Server side render the application, serve static assets, and register server functions
        .serve_dioxus_application("/xx/api", ServeConfigBuilder::new(app, ()))
        .layer(Extension(ss));

    // run it
    #[cfg(not(feature = "dev"))]
    let app = app.layer(
        tower::ServiceBuilder::new().layer(tower_http::compression::CompressionLayer::new()),
    );
    #[cfg(feature = "dev")]
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));

    #[cfg(not(feature = "dev"))]
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    info!("listening: http://{}", addr.to_string());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

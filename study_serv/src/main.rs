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
                    .max_size(10)
                    .min_idle(Some(1))
                    .idle_timeout(Some(Duration::from_secs(170)))
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
                #[cfg(feature = "dev")]
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
                #[cfg(not(feature = "dev"))]
                let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

                info!("listening: http://{}", addr.to_string());
                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });
    }
}

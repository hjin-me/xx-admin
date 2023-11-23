use crate::backend::StateSession;
use crate::state::State;
use anyhow::{anyhow, Result};
use axum::Extension;
use dioxus_fullstack::prelude::extract;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use tokio::time;
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

pub async fn try_get_state() -> Result<State> {
    let Extension(ss): Extension<StateSession> = extract().await?;

    let state = ss.get().await?;

    Ok(state)
}

// Copyright (c) 2026 said885 <frensh5@proton.me>
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NEXUS Relay Server.
//
// NEXUS Relay Server is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// NEXUS Relay Server is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with NEXUS Relay Server. If not, see <https://www.gnu.org/licenses/>.

//! NEXUS Relay Server - Post-Quantum Secure Messaging Relay
//!
//! This server implements a sealed-sender relay for the NEXUS messaging protocol.
//! It never inspects message contents - only routing metadata is processed.

// Strict linting for production code
#![deny(unused_must_use)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![allow(unused_crate_dependencies)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]

pub mod error;
pub mod handler;
pub mod state;
pub mod tls;
pub mod api;
mod metrics;
mod federation;
mod plugins;
mod groups;
mod call_encryption;
mod sync;
mod accounts;
mod push_notifications;
mod message_search;
mod media_storage;
mod persistence;
mod audit;
mod reactions;
mod voice_messages;
mod presence;
mod drafts;
mod rate_limiting;
mod notifications;
mod backup;
mod scheduling;
mod encryption_manager;
mod secure_deletion;
mod metadata_privacy;
mod threat_detection;
mod access_control;
mod ml_threat_detection;
mod differential_privacy;
// New cryptography modules (Phase v0.3.0)
mod challenge_verification;
mod replay_protection;
mod ws_transport_crypto;
mod multicast_groups;
mod envelope_encryption;
mod temporal_messages;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use handler::{
    fetch_prekeys_handler, health_handler, register_handler, upload_prekeys_handler, ws_handler,
};
use state::AppState;

// ---------------------------------------------------------------------------
// Banner
// ---------------------------------------------------------------------------

fn print_banner() {
    println!(
        r#"
  ███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗
  ████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝
  ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗
  ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║
  ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║
  ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝

  NEXUS Relay  v{}  –  Post-Quantum Messaging Relay
  Sealed-sender  |  No message content stored  |  Hash-only routing
"#,
        env!("CARGO_PKG_VERSION")
    );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Initialise structured logging.  Respect the RUST_LOG environment
    // variable; default to info-level for nexus_relay and warn for everything
    // else to avoid noise from tower/hyper internals.
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "nexus_relay=info,tower_http=warn,axum=warn".parse().unwrap()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    print_banner();

    // Track server start time
    let start_time = Instant::now();

    // Build shared application state.
    let state = AppState::new();

    // Initialize metrics
    let metrics = metrics::Metrics::new().expect("Failed to initialize metrics");

    // Spawn background cleanup task – runs every 5 minutes.
    {
        let state_bg = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                state_bg.cleanup_expired();
            }
        });
    }

    // Spawn metrics update task – runs every 30 seconds.
    {
        let metrics_bg = metrics.clone();
        let state_bg = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                metrics_bg.connected_clients.set(state_bg.clients.len() as f64);
            }
        });
    }

    // CRITICAL SECURITY: CORS is restrictive by default
    // In production, set NEXUS_CORS_ORIGIN env var to your frontend domain
    // e.g.: export NEXUS_CORS_ORIGIN=https://chat.example.com
    let cors = if cfg!(debug_assertions) {
        // Development: permissive CORS for localhost
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        // Production: strict CORS (no *-origin allowed by default)
        use tower_http::cors::AllowOrigin;
        

        let cors_origin = std::env::var("NEXUS_CORS_ORIGIN")
            .unwrap_or_else(|_| {
                warn!("NEXUS_CORS_ORIGIN not set; CORS disabled in production");
                "NEXUS_CORS_DISABLED".to_string()
            });

        if cors_origin == "NEXUS_CORS_DISABLED" {
            CorsLayer::new() // No CORS - reject all cross-origin requests
        } else {
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(move |origin, _| {
                    origin.to_str().ok() == Some(&cors_origin)
                }))
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    "X-Nexus-Identity".parse().unwrap_or(axum::http::header::USER_AGENT),
                ])
        }
    };

    // Build the router with all endpoints
    let app = Router::new()
        // WebSocket relay endpoint.
        .route("/ws", get(ws_handler))
        // Identity / prekey management.
        .route("/register", post(register_handler))
        .route("/prekeys/:recipient_hash", post(fetch_prekeys_handler))
        .route("/upload_prekeys", post(upload_prekeys_handler))
        // Liveness / readiness probe.
        .route("/health", get(health_handler))
        // Prometheus metrics endpoint.
        .route("/metrics", get(move || async move {
            metrics.metrics_response()
        }))
        // REST API endpoints
        .route("/api/v1/status", get(api::api_status))
        .route("/api/v1/stats", get(move |state: axum::extract::State<Arc<AppState>>| async move {
            let uptime = start_time.elapsed().as_secs();
            let stats = serde_json::json!({
                "connected_clients": state.clients.len(),
                "registered_identities": state.prekeys.len(),
                "queued_messages": state.offline_queue.iter().map(|q| q.value().len()).sum::<usize>(),
                "uptime_seconds": uptime,
                "version": env!("CARGO_PKG_VERSION")
            });
            (axum::http::StatusCode::OK, axum::Json(stats))
        }))
        .route("/api/v1/groups", post(api::api_create_group))
        .route("/api/v1/groups/:id", get(api::api_get_group))
        .route("/api/v1/calls", post(api::api_initiate_call))
        .route("/api/v1/calls/:id", get(api::api_get_call))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Bind address
    let bind_addr: SocketAddr = std::env::var("NEXUS_LISTEN")
        .unwrap_or_else(|_| "0.0.0.0:8443".to_owned())
        .parse()
        .expect("NEXUS_LISTEN must be a valid socket address");

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind {bind_addr}: {e}"));

    info!(addr = %bind_addr, "NEXUS Relay listening");

    // Check if TLS is configured
    let has_tls = std::env::var("NEXUS_TLS_CERT").is_ok() && std::env::var("NEXUS_TLS_KEY").is_ok();
    if has_tls {
        info!("TLS environment variables found. Use a reverse proxy (nginx, caddy) for production TLS.");
    }

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");

    info!("NEXUS Relay shut down cleanly");
}

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("Received Ctrl-C, shutting down...");
        },
        _ = terminate => {
            warn!("Received SIGTERM, shutting down...");
        },
    }
}

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod session;
pub use session::SessionStore;
pub type SharedState = Arc<AppState>;
pub struct AppState {
    pub sessions: SessionStore,
    pub frontend_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "flashcut_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState {
        sessions: SessionStore::new(),
        frontend_url: std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8080".into()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/sessions", post(handlers::create_session))
        .route("/api/sessions/:id", get(handlers::get_session))
        .route("/ws/:session_id", get(handlers::ws_handler))
        .route("/health", get(|| async { "OK" }))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: std::net::SocketAddr = "0.0.0.0:3001".parse()?;
    info!("FlashCut Backend läuft auf http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

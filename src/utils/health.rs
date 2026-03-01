use axum::{routing::get, Router};
use std::net::SocketAddr;

pub async fn start_health_server() {
    let app = Router::new().route("/health", get(|| async { "OK" }));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

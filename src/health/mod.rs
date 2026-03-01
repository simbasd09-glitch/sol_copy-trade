use tokio::sync::oneshot;
use warp::Filter;
use tracing::info;

/// Spawn a simple health HTTP server on port 8080.
/// Returns a `oneshot::Sender<()>` that can be used to trigger graceful shutdown.
pub fn spawn_health_server() -> oneshot::Sender<()> {
    let (tx, rx) = oneshot::channel::<()>();

    let health = warp::path!("health").map(|| warp::reply::with_status("ok", warp::http::StatusCode::OK));

    tokio::spawn(async move {
        info!("Health server running on 0.0.0.0:8080");
        let (_addr, server) = warp::serve(health)
            .bind_with_graceful_shutdown(([0, 0, 0, 0], 8080), async {
                let _ = rx.await;
                info!("Health server shutting down");
            });
        server.await;
    });

    tx
}

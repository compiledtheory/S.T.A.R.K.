use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::services::ServeDir;

pub async fn start(tx: broadcast::Sender<String>) {
    let app = Router::new()
        .route("/api/map/stream", get(sse_handler))
        .with_state(tx)
        .fallback_service(ServeDir::new("dist"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("S.T.A.R.K. Axum server running on port 3000");
    axum::serve(listener, app).await.unwrap();
}

async fn sse_handler(
    State(tx): State<broadcast::Sender<String>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe())
        .filter_map(|result| match result {
                Ok(payload) => Some(Ok(Event::default().data(payload))),
                Err(_) => None,
            }
        );

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
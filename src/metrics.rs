use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct MetricsState {
    pub req_2xx: AtomicU64,
    pub req_4xx: AtomicU64,
    pub req_5xx: AtomicU64,
    pub start_time: Instant,
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            req_2xx: AtomicU64::new(0),
            req_4xx: AtomicU64::new(0),
            req_5xx: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn metrics_handler(State(state): State<Arc<MetricsState>>) -> impl IntoResponse {
    let req_2xx = state.req_2xx.load(Ordering::Relaxed);
    let req_4xx = state.req_4xx.load(Ordering::Relaxed);
    let req_5xx = state.req_5xx.load(Ordering::Relaxed);
    let uptime = state.start_time.elapsed().as_secs();

    let body = format!(
        "# HELP blist_requests_total Total HTTP requests\n\
         # TYPE blist_requests_total counter\n\
         blist_requests_total{{status=\"2xx\"}} {req_2xx}\n\
         blist_requests_total{{status=\"4xx\"}} {req_4xx}\n\
         blist_requests_total{{status=\"5xx\"}} {req_5xx}\n\
         # HELP blist_uptime_seconds Application uptime in seconds\n\
         # TYPE blist_uptime_seconds gauge\n\
         blist_uptime_seconds {uptime}\n"
    );

    Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(body)
        .unwrap()
}

pub async fn track_metrics(
    State(state): State<Arc<MetricsState>>,
    req: Request,
    next: Next,
) -> Response {
    let response = next.run(req).await;
    let status = response.status().as_u16();

    if (200..300).contains(&status) {
        state.req_2xx.fetch_add(1, Ordering::Relaxed);
    } else if (400..500).contains(&status) {
        state.req_4xx.fetch_add(1, Ordering::Relaxed);
    } else if (500..600).contains(&status) {
        state.req_5xx.fetch_add(1, Ordering::Relaxed);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::util::ServiceExt;

    #[test]
    fn test_metrics_state_new_and_default() {
        let state = MetricsState::new();
        assert_eq!(state.req_2xx.load(Ordering::Relaxed), 0);
        assert_eq!(state.req_4xx.load(Ordering::Relaxed), 0);
        assert_eq!(state.req_5xx.load(Ordering::Relaxed), 0);
        assert!(state.start_time.elapsed().as_secs() < 5);

        let default_state = MetricsState::default();
        assert_eq!(default_state.req_2xx.load(Ordering::Relaxed), 0);
        assert_eq!(default_state.req_4xx.load(Ordering::Relaxed), 0);
        assert_eq!(default_state.req_5xx.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        let state = Arc::new(MetricsState::new());
        state.req_2xx.store(10, Ordering::Relaxed);
        state.req_4xx.store(2, Ordering::Relaxed);
        state.req_5xx.store(1, Ordering::Relaxed);

        let response = metrics_handler(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain"
        );

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("blist_requests_total{status=\"2xx\"} 10"));
        assert!(body_str.contains("blist_requests_total{status=\"4xx\"} 2"));
        assert!(body_str.contains("blist_requests_total{status=\"5xx\"} 1"));
        assert!(body_str.contains("blist_uptime_seconds"));
    }

    #[tokio::test]
    async fn test_track_metrics_middleware() {
        let state = Arc::new(MetricsState::new());

        let app = Router::new()
            .route("/ok", get(|| async { StatusCode::OK }))
            .route("/created", get(|| async { StatusCode::CREATED }))
            .route("/bad", get(|| async { StatusCode::BAD_REQUEST }))
            .route("/not_found", get(|| async { StatusCode::NOT_FOUND }))
            .route(
                "/error",
                get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            )
            .route("/redirect", get(|| async { StatusCode::SEE_OTHER }))
            .layer(middleware::from_fn_with_state(state.clone(), track_metrics));

        // 2xx responses
        let req = Request::builder().uri("/ok").body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let req = Request::builder()
            .uri("/created")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // 4xx responses
        let req = Request::builder().uri("/bad").body(Body::empty()).unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let req = Request::builder()
            .uri("/not_found")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        // 5xx response
        let req = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // 3xx response (redirect - should not increment 2xx/4xx/5xx)
        let req = Request::builder()
            .uri("/redirect")
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::SEE_OTHER);

        // Assert counter values
        assert_eq!(state.req_2xx.load(Ordering::Relaxed), 2);
        assert_eq!(state.req_4xx.load(Ordering::Relaxed), 2);
        assert_eq!(state.req_5xx.load(Ordering::Relaxed), 1);
    }
}

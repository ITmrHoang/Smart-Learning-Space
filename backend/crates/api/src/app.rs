//! Ghép router và các middleware nền.

use std::time::Duration;

use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::from_fn_with_state,
};
use sls_core::{AppContext, AppError};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{controllers, middleware::require_admin};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
/// Giới hạn kích thước body JSON (upload file sẽ đi thẳng lên Garage, không qua API).
const BODY_LIMIT: usize = 1024 * 1024;

pub fn build_router(ctx: AppContext) -> Router {
    let admin = Router::new()
        .merge(controllers::users::admin_routes())
        .merge(controllers::audit::admin_routes())
        .merge(controllers::catalog::admin_routes())
        .merge(controllers::teachers::admin_routes())
        .merge(controllers::sso::admin_routes())
        .route_layer(from_fn_with_state(ctx.clone(), require_admin));

    let api = Router::new()
        .merge(controllers::health::routes())
        .nest("/auth", controllers::auth::routes().merge(controllers::sso::routes()))
        .merge(controllers::catalog::routes())
        .merge(controllers::teachers::routes())
        .merge(controllers::learning::routes())
        .nest("/teaching", controllers::teaching::routes())
        .nest("/admin", admin)
        .fallback(|| async { AppError::not_found("API không tồn tại.") });

    // Thứ tự layer: request_id → log → CORS → timeout → giới hạn body (ngoài cùng chạy trước).
    let layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &axum::http::Request<_>| {
                    let request_id =
                        req.headers().get(REQUEST_ID).and_then(|v| v.to_str().ok()).unwrap_or("-").to_string();
                    tracing::info_span!("http", method = %req.method(), path = %req.uri().path(), request_id)
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(cors(&ctx))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
        .layer(RequestBodyLimitLayer::new(BODY_LIMIT));

    Router::new().nest("/api", api).layer(layers).with_state(ctx)
}

/// Dev chạy qua proxy của Vite (cùng origin) nên không cần CORS; chỉ bật cho các origin khai báo.
fn cors(ctx: &AppContext) -> CorsLayer {
    let origins: Vec<HeaderValue> = ctx.config.cors_origins.iter().filter_map(|o| o.parse().ok()).collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

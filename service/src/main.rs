use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use public_ip_address::perform_lookup;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::net::TcpListener;
use tracing::{info, warn};
use uuid::Uuid;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

// --------- models ---------

#[derive(Clone)]
struct AppState {
    started_at: std::time::SystemTime,
}

#[derive(Deserialize, ToSchema)]
struct LookupRequest {
    ip: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct LookupResponse {
    ip: String,
    raw: serde_json::Value,
    latency_ms: u128,
    request_id: String,
}

#[derive(Serialize, ToSchema)]
struct HealthResponse {
    status: String,
    uptime_sec: u64,
}

#[derive(Serialize, ToSchema)]
struct MetricsResponse {
    service: String,
    version: String,
    uptime_sec: u64,
}

// --------- OpenAPI ---------

#[derive(OpenApi)]
#[openapi(
    paths(lookup_handler, health_handler, metrics_handler),
    components(
        schemas(
            LookupRequest,
            LookupResponse,
            HealthResponse,
            MetricsResponse
        )
    ),
    tags(
        (name = "adatari-ip", description = "IP Intelligence Service")
    )
)]
struct ApiDoc;

// --------- main ---------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        started_at: std::time::SystemTime::now(),
    });

    let app = Router::new()
        .route("/lookup", post(lookup_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .merge(SwaggerUi::new("/swagger").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();

    info!("Listening on {}", addr);
    info!("Swagger: http://localhost:8080/swagger");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

// --------- handlers ---------

#[utoipa::path(
    post,
    path = "/lookup",
    request_body = LookupRequest,
    responses(
        (status = 200, body = LookupResponse)
    )
)]
async fn lookup_handler(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LookupRequest>,
) -> Result<Json<LookupResponse>, StatusCode> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let start = Instant::now();

    let raw_json = if let Some(ip) = req.ip {
        // === РЕАЛЬНЫЙ LOOKUP ПО ЧУЖОМУ IP ===
        lookup_external_ip(&ip).await?
    } else {
        // fallback: мой public IP
        let res = perform_lookup(None)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        serde_json::to_value(res).unwrap()
    };

    let latency = start.elapsed().as_millis();

    let ip = raw_json
        .get("query")
        .or_else(|| raw_json.get("ip"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("lookup ip={} latency={}ms request_id={}", ip, latency, request_id);

    Ok(Json(LookupResponse {
        ip,
        raw: raw_json,
        latency_ms: latency,
        request_id,
    }))
}

// --------- external lookup ---------

async fn lookup_external_ip(ip: &str) -> Result<serde_json::Value, StatusCode> {
    let url = format!("http://ip-api.com/json/{}?fields=66846719", ip);

    let resp = reqwest::get(url)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(json)
}

// --------- infra ---------

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, body = HealthResponse)
    )
)]
async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.started_at.elapsed().unwrap().as_secs();
    Json(HealthResponse { status: "ok".into(), uptime_sec: uptime })
}

#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, body = MetricsResponse)
    )
)]
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let uptime = state.started_at.elapsed().unwrap().as_secs();
    Json(MetricsResponse {
        service: "adatari-ip-service".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_sec: uptime,
    })
}

// --------- shutdown ---------

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.unwrap();
    warn!("shutdown");
}

//! Protected HTTP server for Prometheus metrics endpoint
//!
//! This module provides a protected /metrics endpoint using Axum with
//! authentication support to ensure metrics are only accessible by authorized systems.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

/// Metrics server configuration
pub struct MetricsServerConfig {
    /// Address to bind the metrics server to
    pub bind_address: SocketAddr,
    /// Optional API key for authentication (if None, no auth required)
    pub api_key: Option<String>,
}

impl Default for MetricsServerConfig {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::from(([0, 0, 0, 0], 9090)),
            api_key: std::env::var("METRICS_API_KEY").ok(),
        }
    }
}

/// Create the metrics router with optional authentication
fn create_metrics_router(handle: PrometheusHandle, api_key: Option<String>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .layer(TraceLayer::new_for_http())
        .with_state((handle, api_key))
}

/// Handler for the /metrics endpoint with optional authentication
async fn metrics_handler(
    State((handle, api_key)): State<(PrometheusHandle, Option<String>)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Check authentication if API key is configured
    if let Some(expected_key) = api_key {
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        match auth_header {
            Some(key) if key == expected_key => {
                // Authentication successful
            }
            _ => {
                warn!("Unauthorized metrics access attempt");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    // Render and return Prometheus metrics
    let metrics = handle.render();
    Ok(metrics.into_response())
}

/// Metrics server that runs the HTTP server
pub struct MetricsServer {
    config: MetricsServerConfig,
}

impl MetricsServer {
    /// Create a new metrics server with default configuration
    pub fn new() -> Self {
        Self {
            config: MetricsServerConfig::default(),
        }
    }

    /// Create a new metrics server with custom configuration
    pub fn with_config(config: MetricsServerConfig) -> Self {
        Self { config }
    }

    /// Start the metrics server
    pub async fn run(self, handle: PrometheusHandle) -> anyhow::Result<()> {
        let app = create_metrics_router(handle, self.config.api_key.clone());
        
        info!(
            "Starting metrics server on {} (authentication: {})",
            self.config.bind_address,
            if self.config.api_key.is_some() {
                "enabled"
            } else {
                "disabled"
            }
        );

        let listener = tokio::net::TcpListener::bind(self.config.bind_address).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

impl Default for MetricsServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_metrics_handler_no_auth() {
        let handle = PrometheusBuilder::new()
            .install()
            .expect("Failed to install Prometheus exporter");

        let app = create_metrics_router(handle, None);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_handler_with_auth_success() {
        let handle = PrometheusBuilder::new()
            .install()
            .expect("Failed to install Prometheus exporter");

        let app = create_metrics_router(handle, Some("test-key".to_string()));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .header("authorization", "Bearer test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_handler_with_auth_failure() {
        let handle = PrometheusBuilder::new()
            .install()
            .expect("Failed to install Prometheus exporter");

        let app = create_metrics_router(handle, Some("test-key".to_string()));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .header("authorization", "Bearer wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_handler_without_auth_header() {
        let handle = PrometheusBuilder::new()
            .install()
            .expect("Failed to install Prometheus exporter");

        let app = create_metrics_router(handle, Some("test-key".to_string()));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

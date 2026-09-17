//! Prometheus metrics instrumentation for the Rug Pull Detector WebSocket server
//!
//! This module provides comprehensive metrics for:
//! - RPC node response latencies
//! - Cache hit ratios
//! - Model inference computation durations
//! - Active WebSocket connections
//! - Error rates

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Duration;

/// Initialize the Prometheus metrics exporter
pub fn init_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()
        .expect("Failed to install Prometheus exporter")
}

/// Metrics for RPC operations
pub mod rpc {
    use metrics::{counter, histogram};

    /// Record a successful RPC call
    pub fn record_rpc_success(chain: &str, operation: &str, duration_ms: f64) {
        histogram!("rpc_duration_ms", duration_ms, "chain" => chain, "operation" => operation);
        counter!("rpc_calls_total", 1, "chain" => chain, "operation" => operation, "status" => "success");
    }

    /// Record a failed RPC call
    pub fn record_rpc_error(chain: &str, operation: &str, error_type: &str) {
        counter!("rpc_calls_total", 1, "chain" => chain, "operation" => operation, "status" => "error", "error_type" => error_type);
        counter!("rpc_errors_total", 1, "chain" => chain, "operation" => operation, "error_type" => error_type);
    }

    /// Record RPC timeout
    pub fn record_rpc_timeout(chain: &str, operation: &str) {
        counter!("rpc_calls_total", 1, "chain" => chain, "operation" => operation, "status" => "timeout");
        counter!("rpc_timeouts_total", 1, "chain" => chain, "operation" => operation);
    }
}

/// Metrics for cache operations
pub mod cache {
    use metrics::{counter, gauge, histogram};

    /// Record a cache hit
    pub fn record_cache_hit(cache_name: &str, operation: &str) {
        counter!("cache_operations_total", 1, "cache" => cache_name, "operation" => operation, "result" => "hit");
        counter!("cache_hits_total", 1, "cache" => cache_name);
    }

    /// Record a cache miss
    pub fn record_cache_miss(cache_name: &str, operation: &str) {
        counter!("cache_operations_total", 1, "cache" => cache_name, "operation" => operation, "result" => "miss");
        counter!("cache_misses_total", 1, "cache" => cache_name);
    }

    /// Record cache entry size
    pub fn record_cache_size(cache_name: &str, size: u64) {
        gauge!("cache_size_bytes", size as f64, "cache" => cache_name);
    }

    /// Record cache eviction
    pub fn record_cache_eviction(cache_name: &str) {
        counter!("cache_evictions_total", 1, "cache" => cache_name);
    }

    /// Calculate and record cache hit ratio
    pub fn record_cache_hit_ratio(cache_name: &str, ratio: f64) {
        gauge!("cache_hit_ratio", ratio, "cache" => cache_name);
    }
}

/// Metrics for model inference
pub mod inference {
    use metrics::{counter, histogram};

    /// Record model inference duration
    pub fn record_inference_duration(model_name: &str, duration_ms: f64) {
        histogram!("inference_duration_ms", duration_ms, "model" => model_name);
    }

    /// Record model inference success
    pub fn record_inference_success(model_name: &str) {
        counter!("inference_total", 1, "model" => model_name, "status" => "success");
    }

    /// Record model inference error
    pub fn record_inference_error(model_name: &str, error_type: &str) {
        counter!("inference_total", 1, "model" => model_name, "status" => "error", "error_type" => error_type);
    }

    /// Record number of transactions processed
    pub fn record_transactions_processed(count: u64) {
        counter!("transactions_processed_total", count);
    }

    /// Record number of events processed
    pub fn record_events_processed(count: u64) {
        counter!("events_processed_total", count);
    }
}

/// Metrics for WebSocket connections
pub mod websocket {
    use metrics::{counter, gauge};

    /// Increment active WebSocket connections
    pub fn increment_active_connections() {
        gauge!("websocket_active_connections", |val| val + 1.0);
    }

    /// Decrement active WebSocket connections
    pub fn decrement_active_connections() {
        gauge!("websocket_active_connections", |val| (val - 1.0).max(0.0));
    }

    /// Record new WebSocket connection
    pub fn record_connection() {
        counter!("websocket_connections_total", 1, "status" => "established");
    }

    /// Record WebSocket disconnection
    pub fn record_disconnection(reason: &str) {
        counter!("websocket_connections_total", 1, "status" => "disconnected", "reason" => reason);
    }

    /// Record WebSocket message received
    pub fn record_message_received(message_type: &str) {
        counter!("websocket_messages_total", 1, "direction" => "received", "type" => message_type);
    }

    /// Record WebSocket message sent
    pub fn record_message_sent(message_type: &str) {
        counter!("websocket_messages_total", 1, "direction" => "sent", "type" => message_type);
    }

    /// Record subscription
    pub fn record_subscription() {
        counter!("websocket_subscriptions_total", 1, "action" => "subscribe");
    }

    /// Record unsubscription
    pub fn record_unsubscription() {
        counter!("websocket_subscriptions_total", 1, "action" => "unsubscribe");
    }
}

/// Metrics for actor system
pub mod actor {
    use metrics::{counter, gauge, histogram};

    /// Record block processing duration
    pub fn record_block_processing_duration(chain: &str, duration_ms: f64) {
        histogram!("block_processing_duration_ms", duration_ms, "chain" => chain);
    }

    /// Record blocks processed
    pub fn record_blocks_processed(chain: &str, count: u64) {
        counter!("blocks_processed_total", count, "chain" => chain);
    }

    /// Record actor queue depth
    pub fn record_queue_depth(chain: &str, depth: usize) {
        gauge!("actor_queue_depth", depth as f64, "chain" => chain);
    }

    /// Record backpressure event
    pub fn record_backpressure(chain: &str) {
        counter!("backpressure_events_total", 1, "chain" => chain);
    }

    /// Record actor timeout
    pub fn record_actor_timeout(chain: &str) {
        counter!("actor_timeouts_total", 1, "chain" => chain);
    }

    /// Record inter-actor message latency
    pub fn record_inter_actor_latency(chain: &str, latency_ms: f64) {
        histogram!("inter_actor_latency_ms", latency_ms, "chain" => chain);
    }
}

/// Metrics for database operations
pub mod database {
    use metrics::{counter, histogram};

    /// Record database query duration
    pub fn record_query_duration(operation: &str, duration_ms: f64) {
        histogram!("database_query_duration_ms", duration_ms, "operation" => operation);
    }

    /// Record database query success
    pub fn record_query_success(operation: &str) {
        counter!("database_queries_total", 1, "operation" => operation, "status" => "success");
    }

    /// Record database query error
    pub fn record_query_error(operation: &str, error_type: &str) {
        counter!("database_queries_total", 1, "operation" => operation, "status" => "error", "error_type" => error_type);
    }

    /// Record active database connections
    pub fn record_active_connections(count: u64) {
        gauge!("database_active_connections", count as f64);
    }

    /// Record database connection pool idle connections
    pub fn record_idle_connections(count: u64) {
        gauge!("database_idle_connections", count as f64);
    }
}

/// Metrics for alerts
pub mod alerts {
    use metrics::{counter, histogram};

    /// Record alert generated
    pub fn record_alert_generated(risk_level: &str, alert_type: &str) {
        counter!("alerts_generated_total", 1, "risk_level" => risk_level, "alert_type" => alert_type);
    }

    /// Record alert broadcast duration
    pub fn record_alert_broadcast_duration(duration_ms: f64) {
        histogram!("alert_broadcast_duration_ms", duration_ms);
    }

    /// Record alerts delivered to subscribers
    pub fn record_alerts_delivered(count: u64) {
        counter!("alerts_delivered_total", count);
    }
}

/// Helper trait for timing operations
pub trait TimedExt {
    fn timed<F, R>(self, operation: F) -> R
    where
        F: FnOnce(Duration) -> R;
}

impl TimedExt for std::time::Instant {
    fn timed<F, R>(self, operation: F) -> R
    where
        F: FnOnce(Duration) -> R,
    {
        let duration = self.elapsed();
        operation(duration)
    }
}

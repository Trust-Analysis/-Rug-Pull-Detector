//! Monitoring and metrics collection for the actor system
//! 
//! This module provides real-time monitoring of inter-actor latencies,
//! backpressure events, and overall system health to ensure the
//! sub-10ms latency requirements are met during network burst traffic.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::chain_actor::{ChainId, InterActorMetrics};

/// Maximum acceptable latency for inter-actor communication (in milliseconds)
const MAX_ACCEPTABLE_LATENCY_MS: f64 = 10.0;

/// Alert thresholds for monitoring
const HIGH_LATENCY_THRESHOLD_MS: f64 = 8.0;
const CRITICAL_LATENCY_THRESHOLD_MS: f64 = 15.0;
const HIGH_QUEUE_DEPTH_THRESHOLD: usize = 800;
const CRITICAL_QUEUE_DEPTH_THRESHOLD: usize = 950;

/// System-wide monitoring data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: u64,
    pub chain_metrics: HashMap<String, ChainMetrics>,
    pub global_latency_p50_ms: f64,
    pub global_latency_p95_ms: f64,
    pub global_latency_p99_ms: f64,
    pub total_messages_processed: u64,
    pub total_timeout_count: u64,
    pub total_backpressure_events: u64,
    pub system_health: SystemHealth,
}

/// Per-chain metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetrics {
    pub chain_id: String,
    pub average_latency_ms: f64,
    pub max_latency_ms: f64,
    pub message_count: u64,
    pub timeout_count: u64,
    pub backpressure_events: u64,
    pub queue_depth: usize,
    pub is_healthy: bool,
    pub alerts: Vec<Alert>,
}

/// Alert types for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub chain_id: String,
    pub message: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighLatency,
    CriticalLatency,
    HighQueueDepth,
    CriticalQueueDepth,
    TimeoutSpike,
    BackpressureSpike,
    ActorUnresponsive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Overall system health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemHealth {
    Healthy,
    Degraded,
    Critical,
}

/// Latency tracker for percentile calculations
#[derive(Debug)]
pub struct LatencyTracker {
    latencies: Vec<f64>,
    max_samples: usize,
}

impl LatencyTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            latencies: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn record(&mut self, latency_ms: f64) {
        self.latencies.push(latency_ms);
        if self.latencies.len() > self.max_samples {
            self.latencies.remove(0);
        }
    }

    pub fn percentile(&self, p: f64) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((p / 100.0) * (sorted.len() as f64 - 1.0)) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    pub fn average(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        self.latencies.iter().sum::<f64>() / self.latencies.len() as f64
    }
}

/// Actor monitor for tracking system health and performance
pub struct ActorMonitor {
    chain_metrics: Arc<RwLock<HashMap<ChainId, InterActorMetrics>>>,
    queue_depths: Arc<RwLock<HashMap<ChainId, usize>>>,
    latency_tracker: Arc<RwLock<LatencyTracker>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    monitoring_interval: Duration,
}

impl ActorMonitor {
    /// Create a new actor monitor
    pub fn new(monitoring_interval_ms: u64) -> Self {
        Self {
            chain_metrics: Arc::new(RwLock::new(HashMap::new())),
            queue_depths: Arc::new(RwLock::new(HashMap::new())),
            latency_tracker: Arc::new(RwLock::new(LatencyTracker::new(1000))),
            alerts: Arc::new(RwLock::new(Vec::new())),
            monitoring_interval: Duration::from_millis(monitoring_interval_ms),
        }
    }

    /// Update metrics for a specific chain
    pub async fn update_chain_metrics(&self, chain_id: ChainId, metrics: InterActorMetrics) {
        let mut chain_metrics = self.chain_metrics.write().await;
        chain_metrics.insert(chain_id, metrics);
        
        // Update global latency tracker
        let avg_latency = metrics.average_latency_ms();
        let mut latency_tracker = self.latency_tracker.write().await;
        latency_tracker.record(avg_latency);
        
        // Check for latency alerts
        if avg_latency > CRITICAL_LATENCY_THRESHOLD_MS {
            self.create_alert(
                AlertType::CriticalLatency,
                AlertSeverity::Critical,
                chain_id,
                format!("Critical latency detected: {:.2}ms", avg_latency),
            ).await;
        } else if avg_latency > HIGH_LATENCY_THRESHOLD_MS {
            self.create_alert(
                AlertType::HighLatency,
                AlertSeverity::Warning,
                chain_id,
                format!("High latency detected: {:.2}ms", avg_latency),
            ).await;
        }
    }

    /// Update queue depth for a specific chain
    pub async fn update_queue_depth(&self, chain_id: ChainId, depth: usize) {
        let mut queue_depths = self.queue_depths.write().await;
        queue_depths.insert(chain_id, depth);
        
        // Check for queue depth alerts
        if depth > CRITICAL_QUEUE_DEPTH_THRESHOLD {
            self.create_alert(
                AlertType::CriticalQueueDepth,
                AlertSeverity::Critical,
                chain_id,
                format!("Critical queue depth: {}", depth),
            ).await;
        } else if depth > HIGH_QUEUE_DEPTH_THRESHOLD {
            self.create_alert(
                AlertType::HighQueueDepth,
                AlertSeverity::Warning,
                chain_id,
                format!("High queue depth: {}", depth),
            ).await;
        }
    }

    /// Create and store an alert
    async fn create_alert(
        &self,
        alert_type: AlertType,
        severity: AlertSeverity,
        chain_id: ChainId,
        message: String,
    ) {
        let alert = Alert {
            alert_type,
            severity,
            chain_id: chain_id.as_str().to_string(),
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: HashMap::new(),
        };
        
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);
        
        // Keep only last 100 alerts
        if alerts.len() > 100 {
            alerts.remove(0);
        }
    }

    /// Get current system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        let chain_metrics = self.chain_metrics.read().await;
        let queue_depths = self.queue_depths.read().await;
        let latency_tracker = self.latency_tracker.read().await;
        let alerts = self.alerts.read().await;
        
        let mut chain_metrics_map = HashMap::new();
        let mut total_messages = 0u64;
        let mut total_timeouts = 0u64;
        let mut total_backpressure = 0u64;
        let mut all_latencies = Vec::new();
        
        for (chain_id, metrics) in chain_metrics.iter() {
            let queue_depth = queue_depths.get(chain_id).copied().unwrap_or(0);
            
            // Calculate chain-specific alerts
            let chain_alerts: Vec<Alert> = alerts.iter()
                .filter(|a| a.chain_id == chain_id.as_str())
                .cloned()
                .collect();
            
            let is_healthy = metrics.average_latency_ms() < MAX_ACCEPTABLE_LATENCY_MS 
                && queue_depth < HIGH_QUEUE_DEPTH_THRESHOLD;
            
            chain_metrics_map.insert(
                chain_id.as_str().to_string(),
                ChainMetrics {
                    chain_id: chain_id.as_str().to_string(),
                    average_latency_ms: metrics.average_latency_ms(),
                    max_latency_ms: metrics.max_latency_ms(),
                    message_count: metrics.message_count,
                    timeout_count: metrics.timeout_count,
                    backpressure_events: metrics.backpressure_events,
                    queue_depth,
                    is_healthy,
                    alerts: chain_alerts,
                }
            );
            
            total_messages += metrics.message_count;
            total_timeouts += metrics.timeout_count;
            total_backpressure += metrics.backpressure_events;
            all_latencies.push(metrics.average_latency_ms());
        }
        
        let global_latency_p50 = latency_tracker.percentile(50.0);
        let global_latency_p95 = latency_tracker.percentile(95.0);
        let global_latency_p99 = latency_tracker.percentile(99.0);
        
        // Determine overall system health
        let system_health = if global_latency_p95 > CRITICAL_LATENCY_THRESHOLD_MS 
            || total_timeouts > 100 {
            SystemHealth::Critical
        } else if global_latency_p95 > HIGH_LATENCY_THRESHOLD_MS 
            || total_timeouts > 10 {
            SystemHealth::Degraded
        } else {
            SystemHealth::Healthy
        };
        
        SystemMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chain_metrics: chain_metrics_map,
            global_latency_p50_ms: global_latency_p50,
            global_latency_p95_ms: global_latency_p95,
            global_latency_p99_ms: global_latency_p99,
            total_messages_processed: total_messages,
            total_timeout_count: total_timeouts,
            total_backpressure_events: total_backpressure,
            system_health,
        }
    }

    /// Get recent alerts
    pub async fn get_recent_alerts(&self, limit: usize) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.iter().rev().take(limit).cloned().collect()
    }

    /// Start continuous monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting actor monitoring with interval {:?}", self.monitoring_interval);
        
        let chain_metrics = self.chain_metrics.clone();
        let queue_depths = self.queue_depths.clone();
        let alerts = self.alerts.clone();
        let interval = self.monitoring_interval;
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                // Perform periodic health checks
                let metrics = chain_metrics.read().await;
                let depths = queue_depths.read().await;
                
                for (chain_id, metrics) in metrics.iter() {
                    let depth = depths.get(chain_id).copied().unwrap_or(0);
                    
                    // Check for timeout spikes
                    if metrics.timeout_count > 0 && metrics.message_count > 0 {
                        let timeout_rate = metrics.timeout_count as f64 / metrics.message_count as f64;
                        if timeout_rate > 0.01 { // More than 1% timeout rate
                            warn!("High timeout rate for {}: {:.2}%", 
                                  chain_id.as_str(), timeout_rate * 100.0);
                        }
                    }
                    
                    // Check for backpressure spikes
                    if metrics.backpressure_events > 10 {
                        warn!("High backpressure events for {}: {}", 
                              chain_id.as_str(), metrics.backpressure_events);
                    }
                    
                    debug!("Health check for {}: latency={:.2}ms, queue_depth={}, timeouts={}, backpressure={}",
                           chain_id.as_str(),
                           metrics.average_latency_ms(),
                           depth,
                           metrics.timeout_count,
                           metrics.backpressure_events);
                }
            }
        });
        
        Ok(())
    }

    /// Generate a health report
    pub async fn generate_health_report(&self) -> String {
        let metrics = self.get_system_metrics().await;
        let recent_alerts = self.get_recent_alerts(10).await;
        
        let mut report = format!(
            "=== Actor System Health Report ===\n\
             Timestamp: {}\n\
             System Health: {:?}\n\
             Global Latency (p50/p95/p99): {:.2}ms / {:.2}ms / {:.2}ms\n\
             Total Messages: {}\n\
             Total Timeouts: {}\n\
             Total Backpressure Events: {}\n\n",
            metrics.timestamp,
            metrics.system_health,
            metrics.global_latency_p50_ms,
            metrics.global_latency_p95_ms,
            metrics.global_latency_p99_ms,
            metrics.total_messages_processed,
            metrics.total_timeout_count,
            metrics.total_backpressure_events
        );
        
        report.push_str("Per-Chain Metrics:\n");
        for (chain_id, chain_metrics) in &metrics.chain_metrics {
            report.push_str(&format!(
                "  {}:\n\
                    Latency: {:.2}ms (avg), {:.2}ms (max)\n\
                    Queue Depth: {}\n\
                    Messages: {}\n\
                    Timeouts: {}\n\
                    Backpressure: {}\n\
                    Healthy: {}\n",
                chain_id,
                chain_metrics.average_latency_ms,
                chain_metrics.max_latency_ms,
                chain_metrics.queue_depth,
                chain_metrics.message_count,
                chain_metrics.timeout_count,
                chain_metrics.backpressure_events,
                chain_metrics.is_healthy
            ));
        }
        
        if !recent_alerts.is_empty() {
            report.push_str("\nRecent Alerts:\n");
            for alert in &recent_alerts {
                report.push_str(&format!(
                    "  [{:?}] {} - {}: {}\n",
                    alert.severity,
                    alert.chain_id,
                    format!("{:?}", alert.alert_type),
                    alert.message
                ));
            }
        }
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_actor::InterActorMetrics;

    #[tokio::test]
    async fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new(10);
        
        tracker.record(5.0);
        tracker.record(10.0);
        tracker.record(15.0);
        
        assert_eq!(tracker.percentile(50.0), 10.0);
        assert_eq!(tracker.average(), 10.0);
    }

    #[tokio::test]
    async fn test_monitor_update() {
        let monitor = ActorMonitor::new(100);
        
        let mut metrics = InterActorMetrics::new();
        metrics.record_latency(5_000_000); // 5ms in nanoseconds
        
        monitor.update_chain_metrics(ChainId::Ethereum, metrics).await;
        monitor.update_queue_depth(ChainId::Ethereum, 100).await;
        
        let system_metrics = monitor.get_system_metrics().await;
        assert!(system_metrics.chain_metrics.contains_key("ethereum"));
    }

    #[tokio::test]
    async fn test_alert_creation() {
        let monitor = ActorMonitor::new(100);
        
        let mut metrics = InterActorMetrics::new();
        metrics.record_latency(20_000_000); // 20ms - should trigger critical alert
        
        monitor.update_chain_metrics(ChainId::Ethereum, metrics).await;
        
        let alerts = monitor.get_recent_alerts(10).await;
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_health_report() {
        let monitor = ActorMonitor::new(100);
        
        let mut metrics = InterActorMetrics::new();
        metrics.record_latency(5_000_000);
        
        monitor.update_chain_metrics(ChainId::Ethereum, metrics).await;
        monitor.update_queue_depth(ChainId::Ethereum, 50).await;
        
        let report = monitor.generate_health_report().await;
        assert!(report.contains("Actor System Health Report"));
        assert!(report.contains("ethereum"));
    }
}
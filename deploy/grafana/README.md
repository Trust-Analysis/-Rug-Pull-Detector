# Grafana Dashboards for Rug Pull Detector

This directory contains pre-configured Grafana dashboard JSON templates for monitoring the Rug Pull Detector WebSocket server metrics.

## Dashboards

### 1. Overview Dashboard (`overview-dashboard.json`)
**UID:** `rug-pull-overview`

Provides a high-level view of system health including:
- Active WebSocket connections
- Total blocks processed
- Cache hit statistics
- RPC error rates
- Block processing latency
- Actor queue depths

### 2. RPC Performance Dashboard (`rpc-performance-dashboard.json`)
**UID:** `rug-pull-rpc`

Detailed RPC node performance metrics:
- RPC error rate gauge
- RPC timeout rate gauge
- RPC latency percentiles (p50, p95, p99)
- RPC success rate by chain
- RPC errors by type
- RPC timeouts by chain

### 3. Cache Performance Dashboard (`cache-performance-dashboard.json`)
**UID:** `rug-pull-cache`

Cache performance and efficiency metrics:
- Risk cache hit ratio gauge
- Cache size in bytes
- Cache hit/miss rate
- Cache eviction rate
- Cache hit ratio over time

### 4. WebSocket & Inference Dashboard (`websocket-inference-dashboard.json`)
**UID:** `rug-pull-websocket-inference`

WebSocket and model inference metrics:
- Active WebSocket connections
- Message rate (sent/received)
- Subscription rate
- Inference latency (p95)
- WebSocket connection events
- Model inference throughput
- Processing throughput (transactions/events)

## Installation

### Prerequisites
- Grafana instance (v8.5+)
- Prometheus instance configured to scrape metrics from the WebSocket server

### Importing Dashboards

1. Navigate to your Grafana instance
2. Go to **Dashboards** → **Import**
3. Choose **Upload JSON file**
4. Select the dashboard JSON file from this directory
5. Configure the Prometheus data source if prompted
6. Click **Import**

### Alternative: Programmatic Import

You can import dashboards programmatically using the Grafana API:

```bash
# Import overview dashboard
curl -X POST \
  http://your-grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @overview-dashboard.json

# Import RPC performance dashboard
curl -X POST \
  http://your-grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @rpc-performance-dashboard.json

# Import cache performance dashboard
curl -X POST \
  http://your-grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @cache-performance-dashboard.json

# Import WebSocket & inference dashboard
curl -X POST \
  http://your-grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @websocket-inference-dashboard.json
```

## Metrics Endpoint Configuration

The WebSocket server exposes Prometheus metrics on a protected endpoint:

- **Endpoint:** `http://localhost:9090/metrics`
- **Authentication:** Bearer token (optional, configured via `METRICS_API_KEY` environment variable)

### Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'rug-pull-detector'
    scrape_interval: 15s
    metrics_path: '/metrics'
    static_configs:
      - targets: ['localhost:9090']
    # If authentication is enabled
    authorization:
      type: Bearer
      credentials: YOUR_METRICS_API_KEY
```

## Available Metrics

### RPC Metrics
- `rpc_calls_total` - Total RPC calls by chain, operation, and status
- `rpc_errors_total` - Total RPC errors by chain, operation, and error type
- `rpc_timeouts_total` - Total RPC timeouts by chain and operation
- `rpc_duration_ms` - RPC call duration histogram

### Cache Metrics
- `cache_operations_total` - Total cache operations by cache, operation, and result
- `cache_hits_total` - Total cache hits by cache
- `cache_misses_total` - Total cache misses by cache
- `cache_size_bytes` - Current cache size in bytes
- `cache_hit_ratio` - Cache hit ratio gauge
- `cache_evictions_total` - Total cache evictions by cache

### Inference Metrics
- `inference_total` - Total inference operations by model and status
- `inference_duration_ms` - Inference duration histogram by model
- `transactions_processed_total` - Total transactions processed
- `events_processed_total` - Total events processed

### WebSocket Metrics
- `websocket_active_connections` - Current active WebSocket connections
- `websocket_connections_total` - Total connection events by status and reason
- `websocket_messages_total` - Total messages by direction and type
- `websocket_subscriptions_total` - Total subscription events by action

### Actor Metrics
- `blocks_processed_total` - Total blocks processed by chain
- `block_processing_duration_ms` - Block processing duration histogram by chain
- `actor_queue_depth` - Current actor queue depth by chain
- `backpressure_events_total` - Total backpressure events by chain
- `actor_timeouts_total` - Total actor timeouts by chain
- `inter_actor_latency_ms` - Inter-actor message latency histogram by chain

### Database Metrics
- `database_queries_total` - Total database queries by operation and status
- `database_query_duration_ms` - Database query duration histogram by operation
- `database_active_connections` - Active database connections
- `database_idle_connections` - Idle database connections

### Alert Metrics
- `alerts_generated_total` - Total alerts generated by risk level and type
- `alert_broadcast_duration_ms` - Alert broadcast duration histogram
- `alerts_delivered_total` - Total alerts delivered to subscribers

## Environment Variables

Configure the metrics server using the following environment variables:

- `METRICS_API_KEY` - Optional Bearer token for protecting the `/metrics` endpoint
- `METRICS_PORT` - Port for the metrics server (default: 9090)

## Customization

All dashboards are fully editable within Grafana. You can:
- Add additional panels
- Modify queries
- Adjust thresholds and alerts
- Change visualization types
- Add variables for dynamic filtering

## Alerting

To set up alerts based on these metrics:

1. Open a dashboard
2. Click the panel you want to alert on
3. Click the **Alert** icon (bell)
4. Configure alert conditions and notifications
5. Save the alert rule

Recommended alert thresholds:
- RPC error rate > 5%
- Cache hit ratio < 70%
- WebSocket active connections = 0 (if service should be available)
- Block processing latency p95 > 1000ms
- Actor queue depth > 800 (80% of capacity)

## Support

For issues or questions about the metrics implementation, refer to the main project documentation or create an issue in the repository.

//! Integration tests for the actor-based concurrency architecture
//! 
//! These tests verify that the actor system meets the acceptance criteria:
//! - Autonomous worker actors for individual chain adapters
//! - Bounded async channels with backpressure handling
//! - Prevention of thread deadlocks
//! - Sub-10ms inter-actor message passing latencies during network burst traffic

use rug_pull_websocket_server::chain_actor::{ActorCoordinator, ChainActor, ChainId, BlockPayload, TransactionData, EventData};
use rug_pull_websocket_server::actor_monitor::ActorMonitor;
use rug_pull_websocket_server::block_ingestion::{BlockIngestionBridge, IngestionEvent, IngestionTransaction, IngestionEventEntry};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const BURST_TEST_BLOCKS: usize = 1000;
const ACCEPTABLE_LATENCY_MS: f64 = 10.0;
const BACKPRESSURE_CHANNEL_CAPACITY: usize = 1000;

#[tokio::test]
async fn test_autonomous_chain_actors() {
    // Test that each chain has its own autonomous worker actor
    let coordinator = Arc::new(ActorCoordinator::new());
    
    let chains = vec![ChainId::Ethereum, ChainId::BnbChain, ChainId::Stellar, ChainId::Solana];
    
    for chain_id in chains.clone() {
        let actor = ChainActor::new(chain_id);
        coordinator.register_actor(chain_id, actor).await;
        
        // Verify actor is registered and running
        let status = coordinator.get_all_status().await;
        assert!(status.contains_key(&chain_id), "Actor for {:?} should be registered", chain_id);
    }
    
    // Give actors time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let status = coordinator.get_all_status().await;
    for chain_id in chains {
        let chain_status = status.get(&chain_id).expect("Chain should have status");
        assert!(chain_status.is_running, "Actor for {:?} should be running", chain_id);
    }
}

#[tokio::test]
async fn test_bounded_channels_with_backpressure() {
    // Test that bounded channels enforce capacity limits
    let coordinator = Arc::new(ActorCoordinator::new());
    let eth_actor = ChainActor::new(ChainId::Ethereum);
    coordinator.register_actor(ChainId::Ethereum, eth_actor).await;
    
    let monitor = Arc::new(ActorMonitor::new(100));
    monitor.start_monitoring().await.unwrap();
    
    // Create a burst of messages that exceeds channel capacity
    let mut send_count = 0;
    let mut backpressure_detected = false;
    
    for i in 0..(BACKPRESSURE_CHANNEL_CAPACITY + 100) {
        let payload = BlockPayload {
            chain_id: ChainId::Ethereum,
            block_number: i as u64,
            block_hash: format!("0x{:064x}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };
        
        match coordinator.route_block(payload).await {
            Ok(_) => send_count += 1,
            Err(_) => {
                backpressure_detected = true;
                break;
            }
        }
    }
    
    // Verify backpressure was triggered
    assert!(backpressure_detected || send_count <= BACKPRESSURE_CHANNEL_CAPACITY, 
            "Backpressure should be triggered when channel capacity is exceeded");
    
    coordinator.shutdown().await;
}

#[tokio::test]
async fn test_deadlock_prevention() {
    // Test that the system prevents deadlocks under heavy load
    let coordinator = Arc::new(ActorCoordinator::new());
    
    let chains = vec![ChainId::Ethereum, ChainId::BnbChain, ChainId::Stellar];
    for chain_id in chains {
        let actor = ChainActor::new(chain_id);
        coordinator.register_actor(chain_id, actor).await;
    }
    
    let monitor = Arc::new(ActorMonitor::new(100));
    monitor.start_monitoring().await.unwrap();
    
    // Simulate heavy concurrent traffic
    let start_time = Instant::now();
    let mut handles = vec![];
    
    for chain_id in chains {
        let coordinator_clone = coordinator.clone();
        let handle = tokio::spawn(async move {
            for i in 0..100 {
                let payload = BlockPayload {
                    chain_id,
                    block_number: i as u64,
                    block_hash: format!("0x{:064x}", i),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    transactions: vec![TransactionData {
                        hash: format!("0x{:064x}", i),
                        from: format!("0x{:040x}", i),
                        to: Some(format!("0x{:040x}", i + 1)),
                        value: "1000000000000000000".to_string(),
                        gas_used: 21000,
                        gas_price: 50_000_000_000,
                        status: true,
                    }],
                    events: vec![EventData {
                        contract_address: format!("0x{:040x}", i),
                        event_signature: "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925".to_string(),
                        topics: vec![format!("0x{:064x}", i)],
                        data: format!("0x{:064x}", i),
                        log_index: 0,
                    }],
                    metadata: HashMap::new(),
                };
                
                // Use timeout to prevent deadlocks
                let timeout_result = timeout(Duration::from_millis(50), async {
                    coordinator_clone.route_block(payload).await
                }).await;
                
                if timeout_result.is_err() {
                    panic!("Deadlock detected: operation timed out");
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete without deadlock");
    }
    
    let elapsed = start_time.elapsed();
    println!("Deadlock prevention test completed in {:?}", elapsed);
    
    // Verify no timeouts occurred in monitoring
    let metrics = monitor.get_system_metrics().await;
    assert!(metrics.total_timeout_count < 10, "Too many timeouts detected, possible deadlock");
    
    coordinator.shutdown().await;
}

#[tokio::test]
async fn test_sub_10ms_inter_actor_latency() {
    // Test that inter-actor message passing maintains sub-10ms latency during burst traffic
    let coordinator = Arc::new(ActorCoordinator::new());
    
    let chains = vec![ChainId::Ethereum, ChainId::BnbChain, ChainId::Stellar, ChainId::Solana];
    for chain_id in chains {
        let actor = ChainActor::new(chain_id);
        coordinator.register_actor(chain_id, actor).await;
    }
    
    let monitor = Arc::new(ActorMonitor::new(100));
    monitor.start_monitoring().await.unwrap();
    
    // Give actors time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Generate burst traffic
    let mut latencies = Vec::with_capacity(BURST_TEST_BLOCKS);
    
    for i in 0..BURST_TEST_BLOCKS {
        let chain_id = chains[i % chains.len()];
        let start_time = Instant::now();
        
        let payload = BlockPayload {
            chain_id,
            block_number: i as u64,
            block_hash: format!("0x{:064x}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };
        
        let result = coordinator.route_block(payload).await;
        assert!(result.is_ok(), "Block routing should succeed");
        
        let latency = start_time.elapsed();
        latencies.push(latency.as_secs_f64() * 1000.0); // Convert to ms
    }
    
    // Wait for processing to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Calculate latency statistics
    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let p50 = sorted_latencies[sorted_latencies.len() / 2];
    let p95 = sorted_latencies[(sorted_latencies.len() as f64 * 0.95) as usize];
    let p99 = sorted_latencies[(sorted_latencies.len() as f64 * 0.99) as usize];
    let max = *sorted_latencies.last().unwrap();
    
    println!("Latency statistics:");
    println!("  P50: {:.2}ms", p50);
    println!("  P95: {:.2}ms", p95);
    println!("  P99: {:.2}ms", p99);
    println!("  Max: {:.2}ms", max);
    
    // Verify sub-10ms latency requirement
    assert!(p50 < ACCEPTABLE_LATENCY_MS, "P50 latency should be below 10ms");
    assert!(p95 < ACCEPTABLE_LATENCY_MS * 2.0, "P95 latency should be reasonable");
    
    // Check monitor metrics
    let metrics = monitor.get_system_metrics().await;
    println!("Monitor metrics:");
    println!("  Global P50: {:.2}ms", metrics.global_latency_p50_ms);
    println!("  Global P95: {:.2}ms", metrics.global_latency_p95_ms);
    println!("  Global P99: {:.2}ms", metrics.global_latency_p99_ms);
    
    assert!(metrics.global_latency_p50_ms < ACCEPTABLE_LATENCY_MS, 
            "Global P50 latency should be below 10ms");
    
    coordinator.shutdown().await;
}

#[tokio::test]
async fn test_block_ingestion_bridge() {
    // Test the bridge between event ingestion and actor system
    let coordinator = Arc::new(ActorCoordinator::new());
    let eth_actor = ChainActor::new(ChainId::Ethereum);
    coordinator.register_actor(ChainId::Ethereum, eth_actor).await;
    
    let bridge = Arc::new(BlockIngestionBridge::new(coordinator.clone()));
    
    // Create ingestion event
    let event = IngestionEvent {
        chain: "ethereum".to_string(),
        block_number: 100,
        block_hash: "0x123".to_string(),
        timestamp: 1234567890,
        transactions: vec![
            IngestionTransaction {
                hash: "0xabc".to_string(),
                from: "0xdef".to_string(),
                to: Some("0x456".to_string()),
                value: "1000000000000000000".to_string(),
                gas_used: Some(21000),
                gas_price: Some(50_000_000_000),
                status: Some(true),
            }
        ],
        events: vec![
            IngestionEventEntry {
                contract_address: "0x789".to_string(),
                event_signature: "0xtest".to_string(),
                topics: vec!["0xtopic1".to_string()],
                data: "0xdata".to_string(),
                log_index: Some(0),
            }
        ],
        metadata: HashMap::new(),
    };
    
    // Process event through bridge
    let result = bridge.process_event(event).await;
    assert!(result.is_ok(), "Event processing should succeed");
    
    // Check statistics
    let stats = bridge.get_stats().await;
    assert_eq!(stats.total_events_received, 1);
    assert_eq!(stats.successful_conversions, 1);
    assert_eq!(stats.failed_conversions, 0);
    
    coordinator.shutdown().await;
}

#[tokio::test]
async fn test_burst_traffic_handling() {
    // Test system behavior under burst traffic conditions
    let coordinator = Arc::new(ActorCoordinator::new());
    
    let chains = vec![ChainId::Ethereum, ChainId::BnbChain, ChainId::Stellar];
    for chain_id in chains {
        let actor = ChainActor::new(chain_id);
        coordinator.register_actor(chain_id, actor).await;
    }
    
    let monitor = Arc::new(ActorMonitor::new(100));
    monitor.start_monitoring().await.unwrap();
    
    let bridge = Arc::new(BlockIngestionBridge::new(coordinator.clone()));
    
    // Generate burst of events
    let mut events = Vec::with_capacity(BURST_TEST_BLOCKS);
    for i in 0..BURST_TEST_BLOCKS {
        let chain_id = chains[i % chains.len()];
        events.push(IngestionEvent {
            chain: chain_id.as_str().to_string(),
            block_number: i as u64,
            block_hash: format!("0x{:064x}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        });
    }
    
    // Process batch
    let start_time = Instant::now();
    let result = bridge.process_batch(events).await;
    assert!(result.is_ok(), "Batch processing should succeed");
    
    let elapsed = start_time.elapsed();
    println!("Burst of {} events processed in {:?}", BURST_TEST_BLOCKS, elapsed);
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    // Check system health
    let metrics = monitor.get_system_metrics().await;
    println!("System health after burst:");
    println!("  Total messages: {}", metrics.total_messages_processed);
    println!("  Total timeouts: {}", metrics.total_timeout_count);
    println!("  Total backpressure: {}", metrics.total_backpressure_events);
    println!("  System health: {:?}", metrics.system_health);
    
    // Verify system remains healthy
    assert!(matches!(metrics.system_health, 
                    rug_pull_websocket_server::actor_monitor::SystemHealth::Healthy | 
                    rug_pull_websocket_server::actor_monitor::SystemHealth::Degraded),
            "System should remain healthy or degraded, not critical");
    
    coordinator.shutdown().await;
}

#[tokio::test]
async fn test_actor_isolation() {
    // Test that actors are properly isolated and don't interfere with each other
    let coordinator = Arc::new(ActorCoordinator::new());
    
    let chains = vec![ChainId::Ethereum, ChainId::BnbChain, ChainId::Stellar, ChainId::Solana];
    for chain_id in chains {
        let actor = ChainActor::new(chain_id);
        coordinator.register_actor(chain_id, actor).await;
    }
    
    // Send traffic to only one chain
    for i in 0..50 {
        let payload = BlockPayload {
            chain_id: ChainId::Ethereum,
            block_number: i as u64,
            block_hash: format!("0x{:064x}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };
        
        coordinator.route_block(payload).await.unwrap();
    }
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Check that only Ethereum actor processed blocks
    let status = coordinator.get_all_status().await;
    let eth_status = status.get(&ChainId::Ethereum).expect("Ethereum should have status");
    assert!(eth_status.blocks_processed > 0, "Ethereum actor should have processed blocks");
    
    for chain_id in chains {
        if chain_id != ChainId::Ethereum {
            let chain_status = status.get(&chain_id).expect("Chain should have status");
            assert_eq!(chain_status.blocks_processed, 0, 
                      "{:?} actor should not have processed blocks", chain_id);
        }
    }
    
    coordinator.shutdown().await;
}
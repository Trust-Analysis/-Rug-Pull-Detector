//! Block ingestion bridge between existing event engine and new actor system
//! 
//! This module bridges the existing event ingestion engine with the new
//! actor-based architecture, converting normalized block events into
//! actor-compatible payloads and routing them through the coordinator.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, error, info, warn};

use crate::chain_actor::{ActorCoordinator, BlockPayload, ChainId, TransactionData, EventData};

/// Normalized event from the existing event ingestion engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionEvent {
    pub chain: String,
    pub block_number: u64,
    pub block_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<IngestionTransaction>,
    pub events: Vec<IngestionEventEntry>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub status: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionEventEntry {
    pub contract_address: String,
    pub event_signature: String,
    pub topics: Vec<String>,
    pub data: String,
    pub log_index: Option<u32>,
}

/// Block ingestion bridge that converts events to actor payloads
pub struct BlockIngestionBridge {
    coordinator: Arc<ActorCoordinator>,
    conversion_stats: Arc<ConversionStats>,
}

/// Statistics for event conversion
#[derive(Debug, Default)]
pub struct ConversionStats {
    pub total_events_received: AtomicU64,
    pub successful_conversions: AtomicU64,
    pub failed_conversions: AtomicU64,
    pub events_by_chain: HashMap<String, AtomicU64>,
}

/// Snapshot of conversion statistics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionStatsSnapshot {
    pub total_events_received: u64,
    pub successful_conversions: u64,
    pub failed_conversions: u64,
    pub events_by_chain: HashMap<String, u64>,
}

impl BlockIngestionBridge {
    /// Create a new block ingestion bridge
    pub fn new(coordinator: Arc<ActorCoordinator>) -> Self {
        Self {
            coordinator,
            conversion_stats: Arc::new(ConversionStats::default()),
        }
    }

    /// Process an ingestion event and route it to the appropriate actor
    pub async fn process_event(&self, event: IngestionEvent) -> Result<()> {
        // Update stats
        self.conversion_stats.total_events_received.fetch_add(1, Ordering::Relaxed);
        
        {
            let chain_count = self.conversion_stats.events_by_chain
                .entry(event.chain.clone())
                .or_insert_with(|| AtomicU64::new(0));
            chain_count.fetch_add(1, Ordering::Relaxed);
        }

        // Convert to block payload
        let payload = self.convert_to_payload(event).await?;

        // Route to coordinator
        self.coordinator.route_block(payload).await?;

        // Update success stats
        self.conversion_stats.successful_conversions.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Convert ingestion event to actor block payload
    async fn convert_to_payload(&self, event: IngestionEvent) -> Result<BlockPayload> {
        let chain_id = ChainId::from_str(&event.chain)
            .map_err(|e| anyhow!("Failed to parse chain ID: {}", e))?;

        let transactions = event.transactions.into_iter()
            .map(|tx| TransactionData {
                hash: tx.hash,
                from: tx.from,
                to: tx.to,
                value: tx.value,
                gas_used: tx.gas_used.unwrap_or(0),
                gas_price: tx.gas_price.unwrap_or(0),
                status: tx.status.unwrap_or(true),
            })
            .collect();

        let events = event.events.into_iter()
            .map(|ev| EventData {
                contract_address: ev.contract_address,
                event_signature: ev.event_signature,
                topics: ev.topics,
                data: ev.data,
                log_index: ev.log_index.unwrap_or(0),
            })
            .collect();

        Ok(BlockPayload {
            chain_id,
            block_number: event.block_number,
            block_hash: event.block_hash,
            timestamp: event.timestamp,
            transactions,
            events,
            metadata: event.metadata,
        })
    }

    /// Get conversion statistics
    pub async fn get_stats(&self) -> ConversionStatsSnapshot {
        let stats = &self.conversion_stats;
        
        let mut events_by_chain = HashMap::new();
        for (chain, count) in &stats.events_by_chain {
            events_by_chain.insert(chain.clone(), count.load(Ordering::Relaxed));
        }
        
        ConversionStatsSnapshot {
            total_events_received: stats.total_events_received.load(Ordering::Relaxed),
            successful_conversions: stats.successful_conversions.load(Ordering::Relaxed),
            failed_conversions: stats.failed_conversions.load(Ordering::Relaxed),
            events_by_chain,
        }
    }

    /// Process a batch of events
    pub async fn process_batch(&self, events: Vec<IngestionEvent>) -> Result<u64> {
        let mut successful = 0u64;
        
        for event in events {
            match self.process_event(event).await {
                Ok(_) => successful += 1,
                Err(e) => {
                    error!("Failed to process event: {}", e);
                    self.conversion_stats.failed_conversions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
        Ok(successful)
    }
}

/// Simulation of existing event ingestion for testing
pub struct EventSimulator {
    chains: Vec<ChainId>,
    block_interval_ms: u64,
}

impl EventSimulator {
    /// Create a new event simulator
    pub fn new(chains: Vec<ChainId>, block_interval_ms: u64) -> Self {
        Self {
            chains,
            block_interval_ms,
        }
    }

    /// Generate a mock ingestion event
    pub fn generate_event(&self, chain_id: ChainId) -> IngestionEvent {
        let block_number = rand::random::<u64>();
        let block_hash = format!("0x{:064x}", rand::random::<u64>());
        
        IngestionEvent {
            chain: chain_id.as_str().to_string(),
            block_number,
            block_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions: vec![
                IngestionTransaction {
                    hash: format!("0x{:064x}", rand::random::<u64>()),
                    from: format!("0x{:040x}", rand::random::<u64>()),
                    to: Some(format!("0x{:040x}", rand::random::<u64>())),
                    value: format!("{}", rand::random::<u64>()),
                    gas_used: Some(21000),
                    gas_price: Some(50_000_000_000),
                    status: Some(true),
                }
            ],
            events: vec![
                IngestionEventEntry {
                    contract_address: format!("0x{:040x}", rand::random::<u64>()),
                    event_signature: "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925".to_string(),
                    topics: vec![
                        format!("0x{:064x}", rand::random::<u64>()),
                        format!("0x{:064x}", rand::random::<u64>()),
                    ],
                    data: format!("0x{:064x}", rand::random::<u64>()),
                    log_index: Some(0),
                }
            ],
            metadata: HashMap::new(),
        }
    }

    /// Start generating events for testing
    pub async fn start_simulation(&self, bridge: Arc<BlockIngestionBridge>) -> Result<()> {
        info!("Starting event simulation for {} chains", self.chains.len());
        
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(self.block_interval_ms));
        
        loop {
            interval.tick().await;
            
            for chain_id in &self.chains {
                let event = self.generate_event(*chain_id);
                if let Err(e) = bridge.process_event(event).await {
                    error!("Simulation event processing failed: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_actor::ActorCoordinator;

    #[tokio::test]
    async fn test_event_conversion() {
        let coordinator = Arc::new(ActorCoordinator::new());
        let bridge = BlockIngestionBridge::new(coordinator);
        
        let event = IngestionEvent {
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "0x123".to_string(),
            timestamp: 1234567890,
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };
        
        let result = bridge.process_event(event).await;
        assert!(result.is_ok());
        
        let stats = bridge.get_stats().await;
        assert_eq!(stats.total_events_received, 1);
        assert_eq!(stats.successful_conversions, 1);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let coordinator = Arc::new(ActorCoordinator::new());
        let bridge = BlockIngestionBridge::new(coordinator);
        
        let events = vec![
            IngestionEvent {
                chain: "ethereum".to_string(),
                block_number: 100,
                block_hash: "0x123".to_string(),
                timestamp: 1234567890,
                transactions: vec![],
                events: vec![],
                metadata: HashMap::new(),
            },
            IngestionEvent {
                chain: "stellar".to_string(),
                block_number: 200,
                block_hash: "0x456".to_string(),
                timestamp: 1234567891,
                transactions: vec![],
                events: vec![],
                metadata: HashMap::new(),
            },
        ];
        
        let result = bridge.process_batch(events).await;
        assert_eq!(result, Ok(2));
        
        let stats = bridge.get_stats().await;
        assert_eq!(stats.total_events_received, 2);
        assert_eq!(stats.successful_conversions, 2);
    }

    #[tokio::test]
    async fn test_invalid_chain() {
        let coordinator = Arc::new(ActorCoordinator::new());
        let bridge = BlockIngestionBridge::new(coordinator);
        
        let event = IngestionEvent {
            chain: "invalid_chain".to_string(),
            block_number: 100,
            block_hash: "0x123".to_string(),
            timestamp: 1234567890,
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };
        
        let result = bridge.process_event(event).await;
        assert!(result.is_err());
        
        let stats = bridge.get_stats().await;
        assert_eq!(stats.failed_conversions, 1);
    }
}
//! Actor-based concurrency architecture for multi-chain block processing
//! 
//! This module implements autonomous worker actors for individual chain adapters,
//! routing normalized block payloads through bounded asynchronous channels with
//! backpressure handling to prevent thread deadlocks and maintain sub-10ms
//! inter-actor message passing latencies during network burst traffic.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn, instrument};

/// Maximum number of messages in bounded channels before backpressure kicks in
const CHANNEL_CAPACITY: usize = 1000;

/// Timeout for actor message processing to prevent deadlocks
const ACTOR_TIMEOUT_MS: u64 = 10;

/// Chain identifiers supported by the actor system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    Ethereum,
    BnbChain,
    Stellar,
    Solana,
}

impl ChainId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainId::Ethereum => "ethereum",
            ChainId::BnbChain => "bnb_chain",
            ChainId::Stellar => "stellar",
            ChainId::Solana => "solana",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ethereum" | "eth" => Ok(ChainId::Ethereum),
            "bnb_chain" | "bnb" | "bsc" => Ok(ChainId::BnbChain),
            "stellar" | "xlm" => Ok(ChainId::Stellar),
            "solana" | "sol" => Ok(ChainId::Solana),
            _ => Err(anyhow!("Unknown chain identifier: {}", s)),
        }
    }
}

/// Normalized block payload that can be processed by any chain actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayload {
    pub chain_id: ChainId,
    pub block_number: u64,
    pub block_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<TransactionData>,
    pub events: Vec<EventData>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub gas_used: u64,
    pub gas_price: u64,
    pub status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub contract_address: String,
    pub event_signature: String,
    pub topics: Vec<String>,
    pub data: String,
    pub log_index: u32,
}

/// Messages that can be sent to chain actors
#[derive(Debug)]
pub enum ChainActorMessage {
    /// Process a new block payload
    ProcessBlock(BlockPayload),
    /// Request current actor status
    GetStatus(mpsc::Sender<ActorStatus>),
    /// Graceful shutdown
    Shutdown,
}

/// Actor status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorStatus {
    pub chain_id: ChainId,
    pub is_running: bool,
    pub blocks_processed: u64,
    pub last_block_number: Option<u64>,
    pub queue_depth: usize,
    pub average_processing_time_ms: f64,
    pub max_processing_time_ms: f64,
    pub last_error: Option<String>,
}

/// Performance metrics for inter-actor communication
#[derive(Debug, Clone)]
pub struct InterActorMetrics {
    pub message_count: u64,
    pub total_latency_ns: u64,
    pub max_latency_ns: u64,
    pub timeout_count: u64,
    pub backpressure_events: u64,
}

impl InterActorMetrics {
    pub fn new() -> Self {
        Self {
            message_count: 0,
            total_latency_ns: 0,
            max_latency_ns: 0,
            timeout_count: 0,
            backpressure_events: 0,
        }
    }

    pub fn record_latency(&mut self, latency_ns: u64) {
        self.message_count += 1;
        self.total_latency_ns += latency_ns;
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
    }

    pub fn record_timeout(&mut self) {
        self.timeout_count += 1;
    }

    pub fn record_backpressure(&mut self) {
        self.backpressure_events += 1;
    }

    pub fn average_latency_ms(&self) -> f64 {
        if self.message_count == 0 {
            0.0
        } else {
            (self.total_latency_ns as f64) / (self.message_count as f64) / 1_000_000.0
        }
    }

    pub fn max_latency_ms(&self) -> f64 {
        self.max_latency_ns as f64 / 1_000_000.0
    }
}

/// Chain actor - autonomous worker for processing blocks from a specific chain
pub struct ChainActor {
    chain_id: ChainId,
    receiver: mpsc::Receiver<ChainActorMessage>,
    sender: mpsc::Sender<ChainActorMessage>,
    metrics: Arc<RwLock<InterActorMetrics>>,
    status: Arc<RwLock<ActorStatus>>,
    semaphore: Arc<Semaphore>,
    blocks_processed: u64,
    last_block_number: Option<u64>,
    processing_times: Vec<Duration>,
}

impl ChainActor {
    /// Create a new chain actor with bounded channels
    pub fn new(chain_id: ChainId) -> Self {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let semaphore = Arc::new(Semaphore::new(CHANNEL_CAPACITY));
        
        let metrics = Arc::new(RwLock::new(InterActorMetrics::new()));
        let status = Arc::new(RwLock::new(ActorStatus {
            chain_id,
            is_running: false,
            blocks_processed: 0,
            last_block_number: None,
            queue_depth: 0,
            average_processing_time_ms: 0.0,
            max_processing_time_ms: 0.0,
            last_error: None,
        }));

        Self {
            chain_id,
            receiver,
            sender,
            metrics,
            status,
            semaphore,
            blocks_processed: 0,
            last_block_number: None,
            processing_times: Vec::with_capacity(100),
        }
    }

    /// Get a clone of the sender for this actor
    pub fn sender(&self) -> mpsc::Sender<ChainActorMessage> {
        self.sender.clone()
    }

    /// Get current actor status
    pub async fn get_status(&self) -> ActorStatus {
        self.status.read().await.clone()
    }

    /// Get inter-actor metrics
    pub async fn get_metrics(&self) -> InterActorMetrics {
        self.metrics.read().await.clone()
    }

    /// Get the current queue depth
    pub async fn queue_depth(&self) -> usize {
        self.sender.semaphore().available_permits()
    }

    /// Run the actor's main processing loop
    #[instrument(skip(self))]
    pub async fn run(mut self) {
        {
            let mut status = self.status.write().await;
            status.is_running = true;
        }
        
        info!("Chain actor started for {}", self.chain_id.as_str());

        while let Some(message) = self.receiver.recv().await {
            match message {
                ChainActorMessage::ProcessBlock(payload) => {
                    self.process_block(payload).await;
                }
                ChainActorMessage::GetStatus(reply) => {
                    let status = self.status.read().await.clone();
                    let _ = reply.send(status).await;
                }
                ChainActorMessage::Shutdown => {
                    info!("Shutdown signal received for {}", self.chain_id.as_str());
                    break;
                }
            }
        }

        {
            let mut status = self.status.write().await;
            status.is_running = false;
        }
        
        info!("Chain actor stopped for {}", self.chain_id.as_str());
    }

    /// Process a single block payload with timeout protection
    #[instrument(skip(self, payload))]
    async fn process_block(&mut self, payload: BlockPayload) {
        let start_time = Instant::now();
        
        // Acquire semaphore permit for backpressure handling
        let _permit = match self.semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                error!("Failed to acquire semaphore for {}", self.chain_id.as_str());
                let mut metrics = self.metrics.write().await;
                metrics.record_backpressure();
                return;
            }
        };

        // Process with timeout to prevent deadlocks
        let processing_result = timeout(
            Duration::from_millis(ACTOR_TIMEOUT_MS),
            self.process_block_internal(&payload)
        ).await;

        let processing_time = start_time.elapsed();
        let latency_ns = processing_time.as_nanos() as u64;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            match processing_result {
                Ok(_) => metrics.record_latency(latency_ns),
                Err(_) => {
                    metrics.record_timeout();
                    warn!("Processing timeout for block {} on {}", 
                          payload.block_number, self.chain_id.as_str());
                }
            }
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.blocks_processed = self.blocks_processed;
            status.last_block_number = Some(payload.block_number);
            status.queue_depth = self.receiver.capacity() - self.receiver.len();
            
            // Update processing time statistics
            self.processing_times.push(processing_time);
            if self.processing_times.len() > 100 {
                self.processing_times.remove(0);
            }
            
            if !self.processing_times.is_empty() {
                let avg: Duration = self.processing_times.iter().sum::<Duration>() 
                    / self.processing_times.len() as u32;
                let max = *self.processing_times.iter().max().unwrap();
                status.average_processing_time_ms = avg.as_secs_f64() * 1000.0;
                status.max_processing_time_ms = max.as_secs_f64() * 1000.0;
            }
        }

        debug!("Processed block {} for {} in {:.2}ms", 
               payload.block_number, self.chain_id.as_str(), 
               processing_time.as_secs_f64() * 1000.0);
    }

    /// Internal block processing logic
    async fn process_block_internal(&mut self, payload: &BlockPayload) {
        // Update block counters
        self.blocks_processed += 1;
        self.last_block_number = Some(payload.block_number);

        // Process transactions
        for tx in &payload.transactions {
            self.process_transaction(tx).await;
        }

        // Process events
        for event in &payload.events {
            self.process_event(event).await;
        }

        // Chain-specific processing could be added here
        match self.chain_id {
            ChainId::Ethereum => self.process_ethereum_specific(payload).await,
            ChainId::BnbChain => self.process_bnb_specific(payload).await,
            ChainId::Stellar => self.process_stellar_specific(payload).await,
            ChainId::Solana => self.process_solana_specific(payload).await,
        }
    }

    async fn process_transaction(&self, tx: &TransactionData) {
        // Transaction processing logic
        debug!("Processing transaction {} on {}", tx.hash, self.chain_id.as_str());
    }

    async fn process_event(&self, event: &EventData) {
        // Event processing logic
        debug!("Processing event {} on {}", event.event_signature, self.chain_id.as_str());
    }

    async fn process_ethereum_specific(&self, payload: &BlockPayload) {
        // Ethereum-specific processing
        debug!("Ethereum-specific processing for block {}", payload.block_number);
    }

    async fn process_bnb_specific(&self, payload: &BlockPayload) {
        // BNB Chain-specific processing
        debug!("BNB Chain-specific processing for block {}", payload.block_number);
    }

    async fn process_stellar_specific(&self, payload: &BlockPayload) {
        // Stellar-specific processing
        debug!("Stellar-specific processing for block {}", payload.block_number);
    }

    async fn process_solana_specific(&self, payload: &BlockPayload) {
        // Solana-specific processing
        debug!("Solana-specific processing for block {}", payload.block_number);
    }
}

/// Actor coordinator that manages multiple chain actors
pub struct ActorCoordinator {
    actors: HashMap<ChainId, ChainActor>,
    sender: mpsc::Sender<CoordinatorMessage>,
    metrics: Arc<RwLock<HashMap<ChainId, InterActorMetrics>>>,
}

/// Messages for the coordinator
#[derive(Debug)]
pub enum CoordinatorMessage {
    /// Register a new chain actor
    RegisterActor(ChainId, mpsc::Sender<ChainActorMessage>),
    /// Route a block payload to the appropriate actor
    RouteBlock(BlockPayload),
    /// Get status of all actors
    GetAllStatus(mpsc::Sender<HashMap<ChainId, ActorStatus>>),
    /// Get metrics for all actors
    GetAllMetrics(mpsc::Sender<HashMap<ChainId, InterActorMetrics>>),
    /// Shutdown all actors
    Shutdown,
}

impl ActorCoordinator {
    /// Create a new actor coordinator
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let metrics = Arc::new(RwLock::new(HashMap::new()));
        
        let mut coordinator = Self {
            actors: HashMap::new(),
            sender,
            metrics,
        };

        // Start the coordinator's background task
        tokio::spawn(coordinator.run_coordinator(receiver));
        
        coordinator
    }

    /// Get the coordinator's sender
    pub fn sender(&self) -> mpsc::Sender<CoordinatorMessage> {
        self.sender.clone()
    }

    /// Register a chain actor with the coordinator
    pub async fn register_actor(&self, chain_id: ChainId, actor: ChainActor) {
        let sender = actor.sender();
        self.actors.insert(chain_id, actor);
        
        let _ = self.sender.send(CoordinatorMessage::RegisterActor(chain_id, sender)).await;
        
        // Start the actor's processing loop
        tokio::spawn(actor.run());
    }

    /// Route a block payload to the appropriate chain actor
    pub async fn route_block(&self, payload: BlockPayload) -> Result<()> {
        self.sender.send(CoordinatorMessage::RouteBlock(payload))
            .await
            .map_err(|e| anyhow!("Failed to route block: {}", e))
    }

    /// Get status of all actors
    pub async fn get_all_status(&self) -> HashMap<ChainId, ActorStatus> {
        let (reply, mut receiver) = mpsc::channel(1);
        let _ = self.sender.send(CoordinatorMessage::GetAllStatus(reply)).await;
        receiver.recv().await.unwrap_or_default()
    }

    /// Get metrics for all actors
    pub async fn get_all_metrics(&self) -> HashMap<ChainId, InterActorMetrics> {
        let (reply, mut receiver) = mpsc::channel(1);
        let _ = self.sender.send(CoordinatorMessage::GetAllMetrics(reply)).await;
        receiver.recv().await.unwrap_or_default()
    }

    /// Shutdown all actors
    pub async fn shutdown(&self) {
        let _ = self.sender.send(CoordinatorMessage::Shutdown).await;
    }

    /// Coordinator's main processing loop
    async fn run_coordinator(mut self, mut receiver: mpsc::Receiver<CoordinatorMessage>) {
        let mut actor_senders: HashMap<ChainId, mpsc::Sender<ChainActorMessage>> = HashMap::new();
        
        info!("Actor coordinator started");

        while let Some(message) = receiver.recv().await {
            match message {
                CoordinatorMessage::RegisterActor(chain_id, sender) => {
                    actor_senders.insert(chain_id, sender);
                    info!("Registered actor for {}", chain_id.as_str());
                }
                CoordinatorMessage::RouteBlock(payload) => {
                    if let Some(sender) = actor_senders.get(&payload.chain_id) {
                        let _ = sender.send(ChainActorMessage::ProcessBlock(payload)).await;
                    } else {
                        warn!("No actor registered for {}", payload.chain_id.as_str());
                    }
                }
                CoordinatorMessage::GetAllStatus(reply) => {
                    let mut status_map = HashMap::new();
                    for (chain_id, actor) in &self.actors {
                        let status = actor.get_status().await;
                        status_map.insert(*chain_id, status);
                    }
                    let _ = reply.send(status_map).await;
                }
                CoordinatorMessage::GetAllMetrics(reply) => {
                    let metrics = self.metrics.read().await.clone();
                    let _ = reply.send(metrics).await;
                }
                CoordinatorMessage::Shutdown => {
                    info!("Coordinator shutdown initiated");
                    for (chain_id, sender) in &actor_senders {
                        let _ = sender.send(ChainActorMessage::Shutdown).await;
                        info!("Sent shutdown to {}", chain_id.as_str());
                    }
                    break;
                }
            }
        }

        info!("Actor coordinator stopped");
    }
}

impl Default for ActorCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chain_actor_creation() {
        let actor = ChainActor::new(ChainId::Ethereum);
        let status = actor.get_status().await;
        assert_eq!(status.chain_id, ChainId::Ethereum);
        assert!(!status.is_running);
    }

    #[tokio::test]
    async fn test_block_processing() {
        let actor = ChainActor::new(ChainId::Ethereum);
        let sender = actor.sender();
        
        let payload = BlockPayload {
            chain_id: ChainId::Ethereum,
            block_number: 100,
            block_hash: "0x123".to_string(),
            timestamp: 1234567890,
            transactions: vec![],
            events: vec![],
            metadata: HashMap::new(),
        };

        let _ = sender.send(ChainActorMessage::ProcessBlock(payload)).await;
        
        // Run the actor briefly
        tokio::spawn(async move {
            actor.run().await;
        });
        
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_coordinator() {
        let coordinator = ActorCoordinator::new();
        
        let eth_actor = ChainActor::new(ChainId::Ethereum);
        coordinator.register_actor(ChainId::Ethereum, eth_actor).await;
        
        let stellar_actor = ChainActor::new(ChainId::Stellar);
        coordinator.register_actor(ChainId::Stellar, stellar_actor).await;
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let status = coordinator.get_all_status().await;
        assert!(status.contains_key(&ChainId::Ethereum));
        assert!(status.contains_key(&ChainId::Stellar));
    }

    #[tokio::test]
    async fn test_backpressure() {
        let actor = ChainActor::new(ChainId::Ethereum);
        let sender = actor.sender();
        
        // Fill the channel beyond capacity
        for i in 0..(CHANNEL_CAPACITY + 10) {
            let payload = BlockPayload {
                chain_id: ChainId::Ethereum,
                block_number: i as u64,
                block_hash: format!("0x{}", i),
                timestamp: 1234567890,
                transactions: vec![],
                events: vec![],
                metadata: HashMap::new(),
            };
            
            if sender.send(ChainActorMessage::ProcessBlock(payload)).await.is_err() {
                // Channel is full, backpressure is working
                break;
            }
        }
    }
}
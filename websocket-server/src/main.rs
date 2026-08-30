use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};
use uuid::Uuid;
use tracing_subscriber::prelude::*;

mod broadcast;
mod subscription;
mod types;
mod chain_actor;
mod actor_monitor;
mod block_ingestion;

use broadcast::AlertBroadcaster;
use subscription::SubscriptionManager;
use types::{Alert, ClientMessage, ServerMessage};
use rug_pull_websocket_server::database::{create_pool, run_migrations};
use rug_pull_websocket_server::risk_cache::RiskCache;
use rug_pull_websocket_server::cpu_pool::CpuPool;
use rug_pull_websocket_server::chain_actor::{ActorCoordinator, ChainActor, ChainId};
use rug_pull_websocket_server::actor_monitor::ActorMonitor;
use rug_pull_websocket_server::block_ingestion::BlockIngestionBridge;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize telemetry with tokio-console support
    let console_layer = console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .spawn();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_filter(tracing_subscriber::EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(console_layer)
        .with(fmt_layer)
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize database connection pool
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/rug_pull_detector".to_string());
    
    let pool = create_pool(&database_url).await?;
    info!("Connected to PostgreSQL database");

    // Run database migrations
    run_migrations(&pool).await?;
    info!("Database migrations completed");

    // Initialize risk cache with 15-minute cache window
    let risk_cache = Arc::new(RiskCache::new(pool, 15));

    // Initialize Rayon CPU thread pool
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let cpu_pool = Arc::new(CpuPool::new(threads));

    // Initialize actor coordinator for multi-chain block processing
    let actor_coordinator = Arc::new(ActorCoordinator::new());
    
    // Initialize actor monitor for performance tracking
    let actor_monitor = Arc::new(ActorMonitor::new(1000)); // 1 second monitoring interval
    actor_monitor.start_monitoring().await?;
    info!("Actor monitor started");
    
    // Register chain actors for supported networks
    let eth_actor = ChainActor::new(ChainId::Ethereum);
    actor_coordinator.register_actor(ChainId::Ethereum, eth_actor).await;
    
    let bnb_actor = ChainActor::new(ChainId::BnbChain);
    actor_coordinator.register_actor(ChainId::BnbChain, bnb_actor).await;
    
    let stellar_actor = ChainActor::new(ChainId::Stellar);
    actor_coordinator.register_actor(ChainId::Stellar, stellar_actor).await;
    
    let solana_actor = ChainActor::new(ChainId::Solana);
    actor_coordinator.register_actor(ChainId::Solana, solana_actor).await;
    
    info!("Actor coordinator initialized with 4 chain actors");

    let subscription_manager = Arc::new(SubscriptionManager::new());
    let alert_broadcaster = Arc::new(AlertBroadcaster::new(subscription_manager.clone(), risk_cache.clone()));

    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("WebSocket server listening on {}", addr);

    // Spawn alert broadcaster task
    let alert_broadcaster_clone = alert_broadcaster.clone();
    tokio::spawn(async move {
        alert_broadcaster_clone.run_broadcast_loop().await;
    });

    while let Ok((stream, addr)) = listener.accept().await {
        let subscription_manager = subscription_manager.clone();
        let alert_broadcaster = alert_broadcaster.clone();
        let cpu_pool = cpu_pool.clone();

        tokio::spawn(async move {
            let client_id = Uuid::new_v4();
            info!("New client connected: {} from {}", client_id, addr);

            let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    error!("Error during WebSocket handshake: {}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();
            subscription_manager.add_client(client_id).await;

            // Handle incoming messages
            let read_task = async {
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let cpu_pool = cpu_pool.clone();
                            let parse_res = cpu_pool.spawn(move || {
                                serde_json::from_str::<ClientMessage>(&text)
                            }).await;

                            if let Ok(Ok(client_msg)) = parse_res {
                                match client_msg {
                                    ClientMessage::Subscribe { address } => {
                                        subscription_manager
                                            .subscribe(client_id, &address)
                                            .await;
                                        info!("Client {} subscribed to {}", client_id, address);
                                    }
                                    ClientMessage::Unsubscribe { address } => {
                                        subscription_manager
                                            .unsubscribe(client_id, &address)
                                            .await;
                                        info!("Client {} unsubscribed from {}", client_id, address);
                                    }
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("Client {} sent close frame", client_id);
                            break;
                        }
                        Err(e) => {
                            error!("Error receiving message: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            };

            // Handle outgoing messages from alert broadcaster
            let mut rx = alert_broadcaster.subscribe();
            let write_task = async {
                while let Ok(alert) = rx.recv().await {
                    if subscription_manager.is_subscribed(client_id, &alert.address).await {
                        let server_msg = ServerMessage::Alert(alert.clone());
                        if let Ok(json) = serde_json::to_string(&server_msg) {
                            if write.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            };

            tokio::select! {
                _ = read_task => {},
                _ = write_task => {},
            }

            subscription_manager.remove_client(client_id).await;
            info!("Client {} disconnected", client_id);
        });
    }

    Ok(())
}

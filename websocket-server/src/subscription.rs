use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::metrics::websocket;

#[derive(Clone)]
pub struct SubscriptionManager {
    clients: Arc<RwLock<HashMap<Uuid, HashSet<String>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_client(&self, client_id: Uuid) {
        let mut clients = self.clients.write().await;
        clients.insert(client_id, HashSet::new());
        websocket::increment_active_connections();
        websocket::record_connection();
    }

    pub async fn remove_client(&self, client_id: Uuid) {
        let mut clients = self.clients.write().await;
        clients.remove(&client_id);
        websocket::decrement_active_connections();
        websocket::record_disconnection("client_removed");
    }

    pub async fn subscribe(&self, client_id: Uuid, address: &str) {
        let mut clients = self.clients.write().await;
        if let Some(subscriptions) = clients.get_mut(&client_id) {
            subscriptions.insert(address.to_lowercase());
        }
        websocket::record_subscription();
    }

    pub async fn unsubscribe(&self, client_id: Uuid, address: &str) {
        let mut clients = self.clients.write().await;
        if let Some(subscriptions) = clients.get_mut(&client_id) {
            subscriptions.remove(&address.to_lowercase());
        }
        websocket::record_unsubscription();
    }

    pub async fn is_subscribed(&self, client_id: Uuid, address: &str) -> bool {
        let clients = self.clients.read().await;
        if let Some(subscriptions) = clients.get(&client_id) {
            subscriptions.contains(&address.to_lowercase())
        } else {
            false
        }
    }

    pub async fn get_subscribers(&self, address: &str) -> Vec<Uuid> {
        let clients = self.clients.read().await;
        let address_lower = address.to_lowercase();
        clients
            .iter()
            .filter(|(_, subs)| subs.contains(&address_lower))
            .map(|(client_id, _)| *client_id)
            .collect()
    }
}

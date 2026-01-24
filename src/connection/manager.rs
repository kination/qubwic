use std::collections::HashMap;
use std::time::Duration;
use quiche::ConnectionId;
use ring::rand::{SystemRandom, SecureRandom};
use super::client::Client;

/// Connection manager for handling multiple QUIC connections
pub struct ClientMap {
    clients: HashMap<ConnectionId<'static>, Client>,
    rng: SystemRandom,
}

impl ClientMap {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            rng: SystemRandom::new(),
        }
    }

    /// Get mutable reference to a client by connection ID
    pub fn get_mut(&mut self, conn_id: &ConnectionId<'static>) -> Option<&mut Client> {
        self.clients.get_mut(conn_id)
    }

    /// Find connection by source ID (for connection migration)
    pub fn find_by_source_id(&mut self, conn_id: &ConnectionId) -> Option<&mut Client> {
        // Find connection where the incoming dcid matches our source_id
        for client in self.clients.values_mut() {
            if client.conn.source_id() == *conn_id {
                return Some(client);
            }
        }
        None
    }

    /// Insert a new client connection
    pub fn insert(&mut self, conn_id: ConnectionId<'static>, client: Client) {
        self.clients.insert(conn_id, client);
    }

    /// Generate a new random connection ID
    pub fn generate_cid(&self) -> ConnectionId<'static> {
        let mut scid_buf = [0u8; quiche::MAX_CONN_ID_LEN];
        self.rng
            .fill(&mut scid_buf)
            .expect("Failed to generate connection ID");
        ConnectionId::from_vec(scid_buf.to_vec())
    }

    /// Check if there are no active connections
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Get the minimum timeout across all connections
    pub fn get_timeout(&self) -> Option<Duration> {
        self.clients
            .values()
            .filter_map(|c| c.conn.timeout())
            .min()
    }

    /// Remove closed connections
    pub fn cleanup(&mut self) {
        self.clients.retain(|_, c| !c.conn.is_closed());
    }

    /// Gracefully close all connections
    pub fn close_all(&mut self) {
        for client in self.clients.values_mut() {
            if !client.conn.is_closed() {
                // Application-level close with code 0 (Success)
                let _ = client.conn.close(true, 0x00, b"Server shutting down");
            }
        }
    }

    /// Iterate over all clients mutably
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Client> {
        self.clients.values_mut()
    }
}

impl Default for ClientMap {
    fn default() -> Self {
        Self::new()
    }
}

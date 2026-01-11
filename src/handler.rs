use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use mio::net::UdpSocket;
use quiche::{ConnectionId, Header, RecvInfo};
use quiche::h3::NameValue;
use ring::rand::{SystemRandom, SecureRandom};
use log::{debug, info};


pub struct Client {
    pub conn: quiche::Connection,
    pub h3_conn: Option<quiche::h3::Connection>,
}

pub struct ClientMap {
    pub clients: HashMap<ConnectionId<'static>, Client>,
    pub rng: SystemRandom
}

impl ClientMap {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            rng: SystemRandom::new(),
        }
    }

    pub fn get_mut(&mut self, conn_id: &ConnectionId<'static>) -> Option<&mut Client> {
        self.clients.get_mut(conn_id)
    }

    pub fn find_by_source_id(&mut self, conn_id: &ConnectionId) -> Option<&mut Client> {
        // Find connection where the incoming dcid matches our source_id
        for client in self.clients.values_mut() {
            if client.conn.source_id() == *conn_id {
                return Some(client);
            }
        }
        None
    }

    pub fn insert(&mut self, conn_id: ConnectionId<'static>, client: Client) {
        self.clients.insert(conn_id, client);
    }

    pub fn get_timeout(&self) -> Option<Duration> {
        self.clients.values()
            .filter_map(|c| c.conn.timeout())
            .min()
    }

    pub fn cleanup(&mut self) {
        self.clients.retain(|_, c| !c.conn.is_closed());
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut Client> {
        self.clients.values_mut()
    }
}

pub fn handle_packet(
    clients: &mut ClientMap,
    _socket: &mut UdpSocket,
    packet: &mut [u8],
    src: SocketAddr,
    local: SocketAddr,
    config: &mut quiche::Config,
) -> anyhow::Result<()> {
    let header = match Header::from_slice(packet, quiche::MAX_CONN_ID_LEN) {
        Ok(v) => v,
        Err(e) => {
            debug!("Parsing header failed: {:?}", e);
            return Ok(());
        }
    };

    let conn_id = header.dcid.clone();

    // Try to find existing connection by dcid or by source_id
    let client = if let Some(client) = clients.get_mut(&conn_id) {
        client
    } else if let Some(client) = clients.find_by_source_id(&conn_id) {
        client
    } else {
        if header.ty != quiche::Type::Initial {
            debug!("Dropping packet: no connection for {:?}", conn_id);
            return Ok(());
        }
        // Create new connection with server-generated scid
        let scid = {
            let mut scid_buf = [0u8; quiche::MAX_CONN_ID_LEN];
            clients.rng.fill(&mut scid_buf).expect("Failed to generate connection ID");
            ConnectionId::from_vec(scid_buf.to_vec())
        };

        let conn = match quiche::accept(
            &scid,
            Some(&conn_id),
            local,
            src,
            config,
        ) {
            Ok(c) => c,
            Err(e) => {
                debug!("Failed to create connection: {:?}", e);
                return Ok(());
            }
        };

        let client = Client { conn, h3_conn: None };
        // Store using our server's scid
        clients.insert(scid.clone(), client);
        clients.get_mut(&scid).unwrap()
    };

    let recv_info = RecvInfo { from: src, to: local };
    let read = client.conn.recv(packet, recv_info)?;

    debug!("Received {} bytes from {}", read, src);

    // Try to create HTTP/3 connection after receiving packet
    if client.conn.is_established() && client.h3_conn.is_none() {
        let mut h3_config = quiche::h3::Config::new()?;
        h3_config.set_max_field_section_size(16384);
        h3_config.set_qpack_max_table_capacity(100);
        h3_config.set_qpack_blocked_streams(100);

        match quiche::h3::Connection::with_transport(&mut client.conn, &h3_config) {
            Ok(h3_conn) => {
                client.h3_conn = Some(h3_conn);
                info!("HTTP/3 connection established");
            }
            Err(quiche::h3::Error::Done) => {
                // Not ready yet
            }
            Err(e) => {
                debug!("Failed to create H3 connection: {:?}", e);
            }
        }
    }

    // Process HTTP/3 events immediately after receiving data
    if let Some(h3) = &mut client.h3_conn {
        handle_http3_streams(h3, &mut client.conn);
    }

    Ok(())
}

pub fn handle_write_and_timer(
    clients: &mut ClientMap,
    socket: &mut UdpSocket,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 65535];

    for client in clients.values_mut() {
        // Handle timeout
        client.conn.on_timeout();

        // egress loop
        loop {
            let (write_len, send_info) = match client.conn.send(&mut buf) {
                Ok(v) => v,
                Err(quiche::Error::Done) => {
                    break;
                }
                Err(e) => {
                    debug!("Error sending packet: {:?}", e);
                    break;
                }
            };

            socket.send_to(&buf[..write_len], send_info.to)?;

            debug!("Sent {} bytes to {}", write_len, send_info.to);
        }
    }

    clients.cleanup();

    Ok(())
}

// TODO: Implement proper HTTP/3 stream handling
fn handle_http3_streams(
    h3_conn: &mut quiche::h3::Connection,
    conn: &mut quiche::Connection,
) {
    loop {
        match h3_conn.poll(conn) {
            Ok((stream_id, quiche::h3::Event::Headers { list, more_frames: _ })) => {
                debug!("Received headers on stream {}: {:?}", stream_id, list);
                let mut path = "/";

                for header in &list {
                    if header.name() == b":path" {
                        path = std::str::from_utf8(header.value()).unwrap_or("/");
                    }
                }

                let (status, content) = if path.starts_with("/api") {
                    (200, b"{\"message\": \"quiche test\", \"status\": \"ok\"}".to_vec())
                } else {
                    (200, b"<h1>Hello, I'm server!</h1>".to_vec())
                };

                // create response header
                let headers = vec![
                    quiche::h3::Header::new(b":status", status.to_string().as_bytes()),
                    quiche::h3::Header::new(b"server", b"quiche"),
                    quiche::h3::Header::new(b"content-length", content.len().to_string().as_bytes()),
                ];

                if let Err(e) = h3_conn.send_response(conn, stream_id, &headers, false) {
                    debug!("Failed to send response headers: {:?}", e);
                    return;
                }

                if let Err(e) = h3_conn.send_body(conn, stream_id, &content, true) {
                    debug!("Failed to send response body: {:?}", e);
                    return;
                }
            }
            Ok((stream_id, quiche::h3::Event::Data)) => {
                debug!("Received data on stream {}", stream_id);
                // Handle data
            }
            Ok((_stream_id, quiche::h3::Event::Finished)) => {
                debug!("Stream finished");
                // Handle stream finish
            }
            Ok((stream_id, quiche::h3::Event::Reset(error_code))) => {
                debug!("Stream {} reset with error code {}", stream_id, error_code);
            }
            Ok((_stream_id, quiche::h3::Event::PriorityUpdate)) => {
                debug!("Priority update received");
            }
            Ok((_stream_id, quiche::h3::Event::GoAway)) => {
                debug!("GoAway received");
            }
            Err(quiche::h3::Error::Done) => {
                break;
            }
            Err(e) => {
                debug!("HTTP/3 error: {:?}", e);
                break;
            }
        }
    }
}

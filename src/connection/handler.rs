use std::net::SocketAddr;
use mio::net::UdpSocket;
use quiche::{Header, RecvInfo};
use log::debug;
use super::{Client, ClientMap};

/// Handle incoming packet and create/update connection
pub fn handle_packet(
    clients: &mut ClientMap,
    _socket: &mut UdpSocket,
    packet: &mut [u8],
    src: SocketAddr,
    local: SocketAddr,
    config: &mut quiche::Config,
    accepting_new: bool,
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

        if !accepting_new {
            debug!("Dropping Initial packet: server is shutting down");
            return Ok(());
        }

        // Create new connection
        create_new_connection(clients, &header, src, local, config)?
    };

    // Receive and process packet
    let recv_info = RecvInfo { from: src, to: local };
    let read = client.conn.recv(packet, recv_info)?;

    debug!("Received {} bytes from {}", read, src);

    // Try to create HTTP/3 connection
    if let Err(e) = client.try_create_h3() {
        debug!("Failed to create H3 connection: {:?}", e);
    }

    // Process HTTP/3 events
    if let Some(h3) = &mut client.h3_conn {
        crate::http3::handle_streams(h3, &mut client.conn);
    }

    Ok(())
}

/// Create a new QUIC connection
fn create_new_connection<'a>(
    clients: &'a mut ClientMap,
    header: &Header,
    src: SocketAddr,
    local: SocketAddr,
    config: &mut quiche::Config,
) -> anyhow::Result<&'a mut Client> {
    // Generate server connection ID
    let scid = clients.generate_cid();

    let conn = quiche::accept(&scid, Some(&header.dcid), local, src, config)
        .map_err(|e| anyhow::anyhow!("Failed to create connection: {:?}", e))?;

    let client = Client::new(conn);
    clients.insert(scid.clone(), client);

    Ok(clients.get_mut(&scid).unwrap())
}

/// Handle connection timeouts and send packets
pub fn handle_write_and_timer(
    clients: &mut ClientMap,
    socket: &mut UdpSocket,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 65535];

    for client in clients.values_mut() {
        // Handle timeout
        client.conn.on_timeout();

        // Egress loop - send packets
        loop {
            let (write_len, send_info) = match client.conn.send(&mut buf) {
                Ok(v) => v,
                Err(quiche::Error::Done) => break,
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

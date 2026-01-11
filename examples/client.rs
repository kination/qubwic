use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use quiche::h3::NameValue;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let server_addr: SocketAddr = "127.0.0.1:4433".to_socket_addrs()?.next().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(server_addr)?;
    socket.set_nonblocking(true)?;

    let local_addr = socket.local_addr()?;

    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;
    config.verify_peer(false); // 자체 서명 인증서 허용
    config.set_application_protos(&[b"h3"])?;
    config.set_max_idle_timeout(5000);
    config.set_max_recv_udp_payload_size(1350);
    config.set_max_send_udp_payload_size(1350);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);

    let scid = quiche::ConnectionId::from_ref(&[0xba; 16]);

    let mut conn = quiche::connect(None, &scid, local_addr, server_addr, &mut config)?;

    println!("Connecting to {}...", server_addr);

    let mut buf = [0u8; 65535];
    let mut out = [0u8; 1350];

    // Initial handshake
    loop {
        let (write, send_info) = match conn.send(&mut out) {
            Ok(v) => v,
            Err(quiche::Error::Done) => break,
            Err(e) => {
                eprintln!("send failed: {:?}", e);
                break;
            }
        };
        socket.send(&out[..write])?;
    }

    let mut h3_conn: Option<quiche::h3::Connection> = None;
    let mut request_sent = false;

    loop {
        // Receive packets
        loop {
            match socket.recv(&mut buf) {
                Ok(len) => {
                    let recv_info = quiche::RecvInfo {
                        from: server_addr,
                        to: local_addr,
                    };
                    match conn.recv(&mut buf[..len], recv_info) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("recv failed: {:?}", e);
                            break;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("socket recv failed: {:?}", e);
                    break;
                }
            }
        }

        if conn.is_closed() {
            println!("Connection closed: {:?}", conn.stats());
            break;
        }

        // Create H3 connection once QUIC is established
        if conn.is_established() && h3_conn.is_none() {
            let h3_config = quiche::h3::Config::new()?;
            h3_conn = Some(quiche::h3::Connection::with_transport(&mut conn, &h3_config)?);
            println!("HTTP/3 connection established!");
        }

        // Send HTTP/3 request
        if let Some(h3) = &mut h3_conn {
            if !request_sent {
                let req = vec![
                    quiche::h3::Header::new(b":method", b"GET"),
                    quiche::h3::Header::new(b":scheme", b"https"),
                    quiche::h3::Header::new(b":authority", b"localhost"),
                    quiche::h3::Header::new(b":path", b"/"),
                ];

                match h3.send_request(&mut conn, &req, true) {
                    Ok(stream_id) => {
                        println!("Sent request on stream {}", stream_id);
                        request_sent = true;
                    }
                    Err(e) => eprintln!("Failed to send request: {:?}", e),
                }
            }

            // Poll for HTTP/3 events
            loop {
                match h3.poll(&mut conn) {
                    Ok((stream_id, quiche::h3::Event::Headers { list, .. })) => {
                        println!("\n=== Response Headers (stream {}) ===", stream_id);
                        for hdr in &list {
                            println!("  {}: {}",
                                String::from_utf8_lossy(hdr.name()),
                                String::from_utf8_lossy(hdr.value()));
                        }
                    }
                    Ok((stream_id, quiche::h3::Event::Data)) => {
                        let mut body = vec![0u8; 4096];
                        while let Ok(len) = h3.recv_body(&mut conn, stream_id, &mut body) {
                            println!("\n=== Response Body ===");
                            println!("{}", String::from_utf8_lossy(&body[..len]));
                        }
                    }
                    Ok((_, quiche::h3::Event::Finished)) => {
                        println!("\n=== Request Complete ===");
                        conn.close(true, 0, b"done")?;
                    }
                    Ok((_, quiche::h3::Event::Reset(_))) |
                    Ok((_, quiche::h3::Event::PriorityUpdate)) |
                    Ok((_, quiche::h3::Event::GoAway)) => {}
                    Err(quiche::h3::Error::Done) => break,
                    Err(e) => {
                        eprintln!("H3 error: {:?}", e);
                        break;
                    }
                }
            }
        }

        // Send pending packets
        loop {
            let (write, _send_info) = match conn.send(&mut out) {
                Ok(v) => v,
                Err(quiche::Error::Done) => break,
                Err(e) => {
                    eprintln!("send failed: {:?}", e);
                    break;
                }
            };
            socket.send(&out[..write])?;
        }

        if conn.is_closed() {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}

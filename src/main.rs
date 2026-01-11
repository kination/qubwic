mod config;
mod handler;
mod tls;

use std::net::SocketAddr;
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use log::error;

const SERVER: Token = Token(0);


fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = config::load("server.toml").expect("Cannot find server config");
    let mut quiche_config = tls::create_quiche_config(
        &config.tls.cert_file,
        &config.tls.key_file,
    )?;

    let addr: SocketAddr = config.server.address.parse().expect("Invalid address");
    let mut socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket");

    let mut poll = Poll::new().expect("Failed to create Poll");
    poll.registry().register(&mut socket, SERVER, Interest::READABLE).expect("Failed to register socket");

    let mut clients = handler::ClientMap::new();

    let mut buf = [0u8; 65535];
    let mut events = Events::with_capacity(1024);

    loop {
        let timeout = clients.get_timeout().unwrap_or(Duration::from_millis(50));

        if let Err(e) = poll.poll(&mut events, Some(timeout)) {
            if e.kind() != std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e.into());
        }

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    loop {
                        match socket.recv_from(&mut buf) {
                            Ok((len, src)) => {
                                if let Err(e) = handler::handle_packet(
                                    &mut clients,
                                    &mut socket,
                                    &mut buf[..len],
                                    src,
                                    addr,
                                    &mut quiche_config,
                                ) {
                                    error!("Failed to handle packet from {}: {}", src, e);
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                error!("Failed to receive packet: {}", e);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        handler::handle_write_and_timer(&mut clients, &mut socket)?;
        clients.cleanup();
    }
}

use qubwic::config;
use qubwic::connection;
use qubwic::tls;

use std::net::SocketAddr;
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use log::{error, info};
use clap::Parser;

const SERVER: Token = Token(0);
const WAKER: Token = Token(1);


fn main() -> anyhow::Result<()> {
    // Load configuration with CLI args and environment variables
    let mut config = match config::load_with_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Configuration Error:\n{}", e);
            std::process::exit(1);
        }
    };

    // Apply CLI overrides
    let args = config::CliArgs::parse();
    if let Some(level) = args.log_level {
        config.logging.level = level;
    }
    if let Some(addr) = args.address {
        config.server.address = addr;
    }
    if let Some(size) = args.read_buffer_size {
        config.server.read_buffer_size = size;
    }
    if let Some(cap) = args.event_capacity {
        config.server.event_capacity = cap;
    }

    // Initialize logger with configured level
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.logging.level)
    ).init();

    log::info!("=========== Starting QuBWic server ===========");
    log::info!("Configuration loaded from: {}", args.config);
    log::info!("Server address: {}", config.server.address);
    log::info!("Worker threads: {}", config.server.worker_threads);
    log::info!("QUIC congestion control: {}", config.quic.congestion_control);
    log::info!("Log level: {}", config.logging.level);
    log::info!("Read buffer size: {}", config.server.read_buffer_size);
    log::info!("Event capacity: {}", config.server.event_capacity);
    log::info!("================================================"); 

    let mut quiche_config = tls::create_quiche_config(&config.tls, &config.quic)?;

    let addr: SocketAddr = config.server.address.parse().expect("Invalid address");
    let mut socket = UdpSocket::bind(addr).expect("Failed to bind UDP socket");

    let mut poll = Poll::new().expect("Failed to create Poll");
    poll.registry().register(&mut socket, SERVER, Interest::READABLE).expect("Failed to register socket");

    // Set up Waker for graceful shutdown
    let waker = std::sync::Arc::new(mio::Waker::new(poll.registry(), WAKER).expect("Failed to create Waker"));
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    
    let s = shutdown.clone();
    let w = waker.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal, starting graceful shutdown...");
        s.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = w.wake();
    }).expect("Error setting Ctrl-C handler");

    let mut clients = connection::ClientMap::new();

    let mut buf = vec![0u8; config.server.read_buffer_size];
    let mut events = Events::with_capacity(config.server.event_capacity);

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
                                if let Err(e) = connection::handle_packet(
                                    &mut clients,
                                    &mut socket,
                                    &mut buf[..len],
                                    src,
                                    addr,
                                    &mut quiche_config,
                                    !shutdown.load(std::sync::atomic::Ordering::SeqCst),
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
                WAKER => {
                    // Waker triggered, check shutdown flag
                }
                _ => {}
            }
        }

        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            clients.close_all();
            // Optional: after calling close_all, we could break eventually
            // but we need to call handle_write_and_timer to send close frames.
            if clients.is_empty() {
                info!("All connections closed. Exiting.");
                break Ok(());
            }
        }

        connection::handle_write_and_timer(&mut clients, &mut socket)?;
        clients.cleanup();
    }
}

use qubwic::config;
use qubwic::connection;
use qubwic::tls;

use std::net::SocketAddr;
use std::time::Duration;

use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;
use log::{error, info, LevelFilter};
use clap::Parser;
use signal_hook::consts::signal::SIGHUP;
use signal_hook_mio::v1_0::Signals;
use std::str::FromStr;

const SERVER: Token = Token(0);
const WAKER: Token = Token(1);
const SIGNAL: Token = Token(2);


fn main() -> anyhow::Result<()> {
    // Load configuration with CLI args and environment variables
    let mut config = match config::load_with_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Configuration Error:\n{}", e);
            std::process::exit(1);
        }
    };

    // Apply CLI overrides logic
    // TODO: CLI args are parsed twice (once for config, once for reload), so optimize later
    //
    // Note: In a real app we might want to store CLI args to re-apply them on reload
    let args = config::CliArgs::parse();
    if let Some(ref level) = args.log_level {
        config.logging.level = level.clone();
    }
    if let Some(ref addr) = args.address {
        config.server.address = addr.clone();
    }
    if let Some(size) = args.read_buffer_size {
        config.server.read_buffer_size = size;
    }
    if let Some(cap) = args.event_capacity {
        config.server.event_capacity = cap;
    }

    // Initialize logger
    //  - Initialize env_logger with TRACE (max) level
    //  - so we can dynamically control the actual output using log::set_max_level later.
    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace) // Allow everything through the logger itself
        .format_timestamp_millis()
        .init();

    let level = LevelFilter::from_str(&config.logging.level).unwrap_or(LevelFilter::Info);
    log::set_max_level(level);

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

    // Signal handling for SIGHUP (Hot Reload)
    let mut signals = Signals::new(&[SIGHUP])?;
    poll.registry().register(&mut signals, SIGNAL, Interest::READABLE)?;

    let mut clients = connection::ClientMap::new();

    let mut buf = vec![0u8; config.server.read_buffer_size];
    let mut events = Events::with_capacity(config.server.event_capacity);

    loop {
        let timeout = clients.get_timeout().unwrap_or(Duration::from_millis(50));

        if let Err(e) = poll.poll(&mut events, Some(timeout)) {
            if e.kind() != std::io::ErrorKind::Interrupted {
                continue;
            }
            // Interrupted by signal if process get here
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
                SIGNAL => {
                    for signal in signals.pending() {
                        match signal {
                            SIGHUP => {
                                info!("Received SIGHUP. Reloading configuration...");
                                match config::load(&args.config) {
                                    Ok(mut new_config) => {
                                        // Re-apply CLI args
                                        if let Some(ref level) = args.log_level {
                                            new_config.logging.level = level.clone();
                                        }
                                        // TODO: 
                                        // Cannot change bound address or buffer size at runtime
                                        // without re-binding socket, which drops packets.
                                        // Only apply "safe" runtime changes.

                                        // Update Log Level and quiche config (temporary)
                                        let new_level = LevelFilter::from_str(&new_config.logging.level).unwrap_or(LevelFilter::Info);
                                        log::set_max_level(new_level);
                                        info!("Log level updated to: {}", new_level);

                                        match tls::create_quiche_config(&new_config.tls, &new_config.quic) {
                                            Ok(qc) => {
                                                quiche_config = qc;
                                                info!("QUIC configuration reloaded successfully.");
                                            }
                                            Err(e) => {
                                                error!("Failed to recreate QUIC config: {}", e);
                                            }
                                        }

                                        // Update global config reference
                                        config = new_config;
                                    }
                                    Err(e) => {
                                        error!("Failed to reload configuration: {}", e);
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                WAKER => {
                    // TODO: Waker triggered, check shutdown flag
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

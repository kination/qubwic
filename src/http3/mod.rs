use quiche::h3::NameValue;
use log::debug;

/// Handle HTTP/3 stream events
pub fn handle_streams(h3_conn: &mut quiche::h3::Connection, conn: &mut quiche::Connection) {
    loop {
        match h3_conn.poll(conn) {
            Ok((stream_id, quiche::h3::Event::Headers { list, more_frames: _ })) => {
                debug!("Received headers on stream {}: {:?}", stream_id, list);
                handle_request(h3_conn, conn, stream_id, &list);
            }
            Ok((stream_id, quiche::h3::Event::Data)) => {
                debug!("Received data on stream {}", stream_id);
                // TODO: Handle request body data
            }
            Ok((_stream_id, quiche::h3::Event::Finished)) => {
                debug!("Stream finished");
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

/// Handle HTTP/3 request
fn handle_request(
    h3_conn: &mut quiche::h3::Connection,
    conn: &mut quiche::Connection,
    stream_id: u64,
    headers: &[quiche::h3::Header],
) {
    let mut path = "/";

    for header in headers {
        if header.name() == b":path" {
            path = std::str::from_utf8(header.value()).unwrap_or("/");
        }
    }

    let (status, content) = route_request(path);

    send_response(h3_conn, conn, stream_id, status, &content);
}

/// Route request to appropriate handler
pub fn route_request(path: &str) -> (u16, Vec<u8>) {
    if path.starts_with("/api") {
        (
            200,
            b"{\"message\": \"quiche test\", \"status\": \"ok\"}".to_vec(),
        )
    } else {
        (200, b"<h1>Hello, I'm server!</h1>".to_vec())
    }
}

/// Send HTTP/3 response
fn send_response(
    h3_conn: &mut quiche::h3::Connection,
    conn: &mut quiche::Connection,
    stream_id: u64,
    status: u16,
    content: &[u8],
) {
    let headers = vec![
        quiche::h3::Header::new(b":status", status.to_string().as_bytes()),
        quiche::h3::Header::new(b"server", b"quiche"),
        quiche::h3::Header::new(b"content-length", content.len().to_string().as_bytes()),
    ];

    if let Err(e) = h3_conn.send_response(conn, stream_id, &headers, false) {
        debug!("Failed to send response headers: {:?}", e);
        return;
    }

    if let Err(e) = h3_conn.send_body(conn, stream_id, content, true) {
        debug!("Failed to send response body: {:?}", e);
    }
}

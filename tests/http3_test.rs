use qubwic::http3;

#[test]
fn test_route_request_api() {
    let (status, body) = http3::route_request("/api/test");
    assert_eq!(status, 200);
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("quiche test"));
}

#[test]
fn test_route_request_root() {
    let (status, body) = http3::route_request("/");
    assert_eq!(status, 200);
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("Hello, I'm server!"));
}

#[test]
fn test_route_request_unknown() {
    // Current logic falls back to root handler for anything not starting with /api
    let (status, body) = http3::route_request("/unknown");
    assert_eq!(status, 200);
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("Hello, I'm server!"));
}

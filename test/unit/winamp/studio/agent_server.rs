use super::*;

fn server() -> (Server, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    (
        Server::bind(
            listener,
            crate::winamp::studio::open_document(None).unwrap(),
        )
        .unwrap(),
        address,
    )
}

#[test]
fn real_connection_probe_is_read_only_and_port_can_be_restarted() {
    let (server, address) = server();
    let shared = server.document.lock().unwrap().clone();
    let before = shared.lock().unwrap().status();
    let (version, count) = probe(address).unwrap();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert!(count > 10);
    assert_eq!(shared.lock().unwrap().status(), before);
    drop(server);
    assert!(TcpStream::connect(address).is_err());
    let restarted = Server::bind(TcpListener::bind(address).unwrap(), shared).unwrap();
    assert!(probe(address).is_ok());
    drop(restarted);
}

#[test]
fn stopping_disconnects_a_client_stalled_in_http_headers() {
    let (server, address) = server();
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(b"POST /mcp HTTP/1.1\r\n").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while server.connection.lock().unwrap().is_none() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let before = std::time::Instant::now();
    drop(server);
    assert!(before.elapsed() < Duration::from_secs(1));
    let mut byte = [0];
    client
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    assert!(matches!(client.read(&mut byte), Ok(0) | Err(_)));
    assert!(TcpListener::bind(address).is_ok());
}

#[test]
fn browser_origin_is_rejected_and_non_loopback_bind_is_refused() {
    let (_server, address) = server();
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    client
        .write_all(
            b"POST /mcp HTTP/1.1\r\nOrigin: https://example.com\r\nContent-Length: 0\r\n\r\n",
        )
        .unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400"));
    assert!(Server::bind(
        TcpListener::bind("0.0.0.0:0").unwrap(),
        crate::winamp::studio::open_document(None).unwrap()
    )
    .is_err());
}
#[test]
fn oversized_headers_and_untrusted_hosts_are_rejected() {
    let (_server, address) = server();
    for request in [
        "POST /mcp HTTP/1.1\r\nHost: attacker.example\r\nContent-Length: 0\r\n\r\n".to_string(),
        format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Length: 0\r\nContent-Length: 2\r\n\r\n"),
        format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nX-Large: {}\r\n\r\n", "x".repeat(HEADER_LIMIT)),
    ] {
        let mut client = TcpStream::connect(address).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        client.write_all(request.as_bytes()).unwrap();
        let mut response = [0; 128];
        let count = client.read(&mut response).unwrap();
        assert!(response[..count].starts_with(b"HTTP/1.1 400"));
    }
}

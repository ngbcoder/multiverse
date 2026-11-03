use multiverse_installer::ConfiguratorServer;
use multiverse_bootcore::config::Config;
use std::io::{Read, Write};
use std::net::TcpStream;

#[test]
fn configurator_http_roundtrip() {
    let cfg = Config::default();
    let mut server = ConfiguratorServer::new(18181, cfg);
    // run server in background thread
    std::thread::spawn(move || {
        let _ = server.run_once();
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect("127.0.0.1:18181").unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("200 OK"));
    assert!(resp.contains("Multiverse Configurator"));
}
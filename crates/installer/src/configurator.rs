//! Local HTTP Configurator Server for live config.json editing and backup.

use multiverse_bootcore::config::Config;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

pub struct ConfiguratorServer {
    pub port: u16,
    pub current_config: Config,
    pub backup_dir: String,
}

impl ConfiguratorServer {
    pub fn new(port: u16, current_config: Config) -> Self {
        Self {
            port,
            current_config,
            backup_dir: ".".to_string(),
        }
    }

    pub fn with_backup_dir(mut self, dir: impl Into<String>) -> Self {
        self.backup_dir = dir.into();
        self
    }

    pub fn handle_update_json(&mut self, json_str: &str) -> Result<String, String> {
        let updated = Config::from_json(json_str).map_err(|e| format!("Invalid JSON config: {}", e))?;
        self.current_config = updated;
        self.current_config.to_json().map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn generate_backup_filename(&self) -> String {
        format!("config.json.bak.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
    }

    pub fn config_json(&self) -> Result<String, String> {
        self.current_config.to_json().map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn html_page(&self) -> String {
        self.render_html()
    }

    pub fn render_html(&self) -> String {
        self.render_page()
    }

    pub fn render_page(&self) -> String {
        let cfg = self.config_json().unwrap_or_else(|e| e);
        format!(
            "<!DOCTYPE html><html><head><title>Multiverse Configurator</title></head><body>\
             <h1>Multiverse Configurator</h1>\
             <form method=\"POST\" action=\"/config\">\
             <textarea name=\"config\" rows=\"20\" cols=\"80\">{}</textarea><br/>\
             <button type=\"submit\">Save</button></form>\
             </body></html>",
            cfg.replace('<', "&lt;").replace('>', "&gt;")
        )
    }

    fn parse_request(stream: &mut TcpStream) -> Result<(String, String, String), String> {
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("bad request line".into());
        }
        let method = parts[0].to_string();
        let path = parts[1].to_string();

        // read headers
        let mut headers = String::new();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|e| e.to_string())?;
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            if line.to_lowercase().starts_with("content-length:") {
                content_length = line.split(':').nth(1).unwrap_or("0").trim().parse().unwrap_or(0);
            }
            headers.push_str(&line);
        }

        // read body
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).map_err(|e| e.to_string())?;
        }
        let body_str = String::from_utf8_lossy(&body).to_string();
        let _ = headers;
        Ok((method, path, body_str))
    }

    pub fn handle_connection(&mut self, stream: TcpStream) -> Result<(), String> {
        self.handle_request(stream)
    }

    pub fn handle_request(&mut self, mut stream: TcpStream) -> Result<(), String> {
        let (method, path, body) = Self::parse_request(&mut stream)?;
        let response = match (method.as_str(), path.as_str()) {
            ("GET", "/") | ("GET", "/index.html") => {
                let html = self.render_page();
                format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}", html.len(), html)
            }
            ("GET", "/config") | ("GET", "/config.json") => {
                match self.config_json() {
                    Ok(json) => format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json),
                    Err(e) => format!("HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\r\n{}", e.len(), e),
                }
            }
            ("POST", "/config") | ("POST", "/update") => {
                // body may be raw json or form-encoded config=...
                let json_payload = if body.trim_start().starts_with('{') {
                    body.clone()
                } else {
                    // try form decode
                    body.split('&').find_map(|pair| {
                        let mut kv = pair.splitn(2, '=');
                        let k = kv.next().unwrap_or("");
                        let v = kv.next().unwrap_or("");
                        if k == "config" { Some(url_decode(v)) } else { None }
                    }).unwrap_or_else(|| body.clone())
                };
                match self.handle_update_json(&json_payload) {
                    Ok(json) => format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json),
                    Err(e) => format!("HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\n\r\n{}", e.len(), e),
                }
            }
            ("GET", "/backup") => {
                let name = self.generate_backup_filename();
                format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", name.len(), name)
            }
            _ => {
                let msg = "Not Found";
                format!("HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{}", msg.len(), msg)
            }
        };
        stream.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn serve_once(&mut self) -> Result<(), String> {
        self.run_once()
    }

    pub fn run_once(&mut self) -> Result<(), String> {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).map_err(|e| e.to_string())?;
        if let Ok((stream, _)) = listener.accept() {
            self.handle_request(stream)?;
        }
        Ok(())
    }

    pub fn serve(&mut self) -> Result<(), String> {
        self.run()
    }

    pub fn run(&mut self) -> Result<(), String> {
        let listener = TcpListener::bind(("0.0.0.0", self.port)).map_err(|e| e.to_string())?;
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let _ = self.handle_request(stream);
                }
                Err(_) => {}
            }
        }
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.run()
    }
}

fn url_decode(input: &str) -> String {
    let mut out = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(' '),
            b'%' if i + 2 < bytes.len() => {
                if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                    out.push(val as char);
                    i += 2;
                } else {
                    out.push(bytes[i] as char);
                }
            }
            _ => out.push(bytes[i] as char),
        }
        i += 1;
    }
    out
}
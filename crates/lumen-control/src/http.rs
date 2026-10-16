use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Response, Server};

use crate::types::{ControlCommand, ControlError, ControlEvent, Source, StatusHandle};

/// Suggested default binding address for the HTTP control server.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:47800";

/// Maximum allowed HTTP request body size (4 KiB = 4096 bytes).
pub const MAX_BODY_SIZE: usize = 4096;

/// Configuration for the HTTP control server.
///
/// Default suggested bind address is `127.0.0.1:47800`.
/// Non-loopback binds are allowed, but note that the bearer token is the only
/// protection and HTTP traffic is unencrypted plaintext.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpConfig {
    /// Address to bind to.
    ///
    /// Non-loopback binds are allowed, but note that the bearer token is the only
    /// protection and HTTP traffic is unencrypted plaintext.
    pub bind: SocketAddr,
    /// Bearer token for authentication. Must be at least 16 characters long.
    pub token: String,
}

impl HttpConfig {
    pub fn new(bind: SocketAddr, token: impl Into<String>) -> Self {
        Self {
            bind,
            token: token.into(),
        }
    }
}

/// Constant-time comparison between two byte slices to mitigate timing attacks.
pub fn constant_time_eq_slices(a: &[u8], b: &[u8]) -> bool {
    let len_diff = if a.len() == b.len() { 0u8 } else { 1u8 };
    let min_len = a.len().min(b.len());
    let mut diff = len_diff;
    for i in 0..min_len {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// HTTP remote control server.
///
/// Runs on its own background thread. Calling [`HttpServer::shutdown`] or dropping
/// the server stops the background thread and joins it cleanly.
pub struct HttpServer {
    server: Option<Arc<Server>>,
    local_addr: Option<SocketAddr>,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HttpServer {
    /// Starts the HTTP control server on a background thread.
    ///
    /// Non-loopback binds are allowed, but note that the bearer token is the only
    /// protection and HTTP traffic is unencrypted plaintext.
    ///
    /// The token must be at least 16 characters; otherwise returns [`ControlError::TokenTooShort`].
    pub fn start(
        config: HttpConfig,
        tx: Sender<ControlEvent>,
        status: StatusHandle,
    ) -> Result<Self, ControlError> {
        let token_len = config.token.chars().count();
        if token_len < 16 {
            return Err(ControlError::TokenTooShort(token_len));
        }

        let server =
            Server::http(config.bind).map_err(|err| ControlError::Http(err.to_string()))?;
        let server = Arc::new(server);
        let running = Arc::new(AtomicBool::new(true));

        let srv = Arc::clone(&server);
        let run = Arc::clone(&running);
        let token = config.token;

        let thread = std::thread::spawn(move || {
            while run.load(Ordering::SeqCst) {
                match srv.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(request)) => {
                        handle_request(request, &token, &tx, &status);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        if !run.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
        });

        let local_addr = server.server_addr().to_ip();

        Ok(Self {
            server: Some(server),
            local_addr,
            running,
            thread: Some(thread),
        })
    }

    /// Returns the local socket address the server is bound to, if available.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    /// Stops the server, closes the listening socket, and joins the background thread.
    pub fn shutdown(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            if let Some(ref server) = self.server {
                server.unblock();
            }
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
            self.server = None;
        }
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn handle_request(
    mut request: tiny_http::Request,
    expected_token: &str,
    tx: &Sender<ControlEvent>,
    status: &StatusHandle,
) {
    // 1. Check Authorization header: Bearer <token> with constant-time comparison
    let auth_header = request.headers().iter().find(|h| {
        h.field
            .as_str()
            .as_str()
            .eq_ignore_ascii_case("Authorization")
    });

    let authorized = match auth_header {
        Some(header) => {
            let expected = format!("Bearer {expected_token}");
            constant_time_eq_slices(header.value.as_str().as_bytes(), expected.as_bytes())
        }
        None => false,
    };

    if !authorized {
        let resp = Response::from_string("Unauthorized").with_status_code(401);
        let _ = request.respond(resp);
        return;
    }

    // 2. Route matching
    let path = request.url().split('?').next().unwrap_or("");
    match (request.method(), path) {
        (&Method::Get, "/status") => {
            let snapshot = status.get();
            match serde_json::to_string(&snapshot) {
                Ok(json) => {
                    let ct =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let resp = Response::from_string(json)
                        .with_header(ct)
                        .with_status_code(200);
                    let _ = request.respond(resp);
                }
                Err(err) => {
                    let resp = Response::from_string(format!("Internal Error: {err}"))
                        .with_status_code(500);
                    let _ = request.respond(resp);
                }
            }
        }
        (&Method::Post, "/command") => {
            if request.body_length().is_some_and(|len| len > MAX_BODY_SIZE) {
                let resp = Response::from_string("Payload Too Large: body exceeds 4 KiB limit")
                    .with_status_code(413);
                let _ = request.respond(resp);
                return;
            }

            let mut body = Vec::new();
            let mut reader = request.as_reader().take((MAX_BODY_SIZE + 1) as u64);
            if reader.read_to_end(&mut body).is_err() || body.len() > MAX_BODY_SIZE {
                let resp = Response::from_string("Payload Too Large: body exceeds 4 KiB limit")
                    .with_status_code(413);
                let _ = request.respond(resp);
                return;
            }

            match serde_json::from_slice::<ControlCommand>(&body) {
                Ok(command) => {
                    if let Err(err) = command.validate() {
                        let resp = Response::from_string(format!("Malformed command: {err}"))
                            .with_status_code(400);
                        let _ = request.respond(resp);
                        return;
                    }
                    let _ = tx.send(ControlEvent {
                        source: Source::Http,
                        command,
                    });
                    let resp = Response::from_string("Accepted").with_status_code(202);
                    let _ = request.respond(resp);
                }
                Err(err) => {
                    let resp = Response::from_string(format!("Malformed command: {err}"))
                        .with_status_code(400);
                    let _ = request.respond(resp);
                }
            }
        }
        _ => {
            let resp = Response::from_string("Not Found").with_status_code(404);
            let _ = request.respond(resp);
        }
    }
}

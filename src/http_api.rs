// HTTP JSON API Server
//
// Provides a REST API for external control of the emulator.
// Runs in a separate thread and communicates with the main loop via channels.

use crate::command::{Command, CommandResult};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use tiny_http::{Method, Response, Server, StatusCode};

/// Request sent from HTTP thread to main loop
pub struct ApiRequest {
    /// The command to execute
    pub command: Command,
    /// Channel to send response back
    pub response_tx: Sender<CommandResult>,
}

/// HTTP API server handle
pub struct HttpApi {
    /// Handle to the server thread
    _thread_handle: JoinHandle<()>,
    /// Receiver for incoming requests
    request_rx: Receiver<ApiRequest>,
}

impl HttpApi {
    /// Start the HTTP API server on the specified address
    ///
    /// Default: "127.0.0.1:8765"
    pub fn start(bind_addr: &str) -> Result<HttpApi, String> {
        let server = Server::http(bind_addr)
            .map_err(|e| format!("Failed to start HTTP server: {}", e))?;

        let (request_tx, request_rx) = channel();

        let thread_handle = thread::spawn(move || {
            run_server(server, request_tx);
        });

        println!("HTTP API listening on http://{}", bind_addr);

        Ok(HttpApi {
            _thread_handle: thread_handle,
            request_rx,
        })
    }

    /// Try to receive a pending API request (non-blocking)
    pub fn try_recv(&self) -> Option<ApiRequest> {
        match self.request_rx.try_recv() {
            Ok(request) => Some(request),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

/// Run the HTTP server (blocking, runs in thread)
fn run_server(server: Server, request_tx: Sender<ApiRequest>) {
    for request in server.incoming_requests() {
        handle_request(request, &request_tx);
    }
}

/// Handle a single HTTP request
fn handle_request(mut request: tiny_http::Request, request_tx: &Sender<ApiRequest>) {
    let path = request.url().to_string();
    let method = request.method().clone();

    match (&method, path.as_str()) {
        (Method::Post, "/command") => {
            // Read body
            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_err() {
                let response = json_response(400, r#"{"error":"Failed to read request body"}"#);
                let _ = request.respond(response);
                return;
            }

            // Parse JSON command
            let command: Command = match serde_json::from_str(&body) {
                Ok(cmd) => cmd,
                Err(e) => {
                    // Try parsing as text command (for convenience)
                    match Command::parse(&body) {
                        Ok(cmd) => cmd,
                        Err(_) => {
                            let error_body = format!(r#"{{"error":"Invalid command: {}"}}"#, e);
                            let response = json_response(400, &error_body);
                            let _ = request.respond(response);
                            return;
                        }
                    }
                }
            };

            execute_and_respond(request, command, request_tx);
        }
        (Method::Get, "/status") => {
            // Shortcut for status command
            execute_and_respond(request, Command::Status, request_tx);
        }
        (Method::Get, "/health") => {
            // Health check endpoint
            let response = json_response(200, r#"{"status":"ok"}"#);
            let _ = request.respond(response);
        }
        (Method::Options, _) => {
            // CORS preflight
            let response = Response::from_string("")
                .with_header(header("Access-Control-Allow-Origin", "*"))
                .with_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
                .with_header(header("Access-Control-Allow-Headers", "Content-Type"))
                .with_status_code(StatusCode(204));
            let _ = request.respond(response);
        }
        _ => {
            // 404 Not Found
            let response = json_response(404, r#"{"error":"Not found"}"#);
            let _ = request.respond(response);
        }
    }
}

/// Execute command and send response
fn execute_and_respond(
    request: tiny_http::Request,
    command: Command,
    request_tx: &Sender<ApiRequest>,
) {
    // Create response channel
    let (response_tx, response_rx) = channel();

    // Send request to main loop
    if request_tx
        .send(ApiRequest {
            command,
            response_tx,
        })
        .is_err()
    {
        let response = json_response(503, r#"{"error":"Server shutting down"}"#);
        let _ = request.respond(response);
        return;
    }

    // Wait for response (with timeout)
    match response_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_else(|_| {
                r#"{"status":"error","message":"Serialization failed"}"#.to_string()
            });
            let response = json_response(200, &json);
            let _ = request.respond(response);
        }
        Err(_) => {
            let response = json_response(504, r#"{"error":"Request timeout"}"#);
            let _ = request.respond(response);
        }
    }
}

/// Create a JSON response with the given status code and body
fn json_response(status: u16, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body)
        .with_header(header("Content-Type", "application/json"))
        .with_header(header("Access-Control-Allow-Origin", "*"))
        .with_status_code(StatusCode(status))
}

/// Create a header from string values
fn header(name: &str, value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
}

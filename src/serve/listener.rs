//! Loopback HTTP listener over the route table.
//!
//! The listener binds 127.0.0.1 only — the substrate is a same-machine
//! product boundary, not a network service — and runs a small pool of
//! worker threads so one blocking long-poll cannot starve other readers.
//! It serves until the handle shuts it down or drops; the foreground
//! process that owns the stores owns the listener's lifetime (DBG-013: a
//! process serving loopback reads and exiting with its session).

use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Response, Server};

use crate::harness::boot::HarnessError;
use crate::serve::routes::{dispatch, RouteResponse};
use crate::serve::sources::ServeSources;

/// Worker threads per listener; long-polls block one worker each.
const WORKER_THREADS: usize = 4;

/// Poll interval for shutdown checks between accepted requests.
const ACCEPT_TIMEOUT: Duration = Duration::from_millis(100);

/// Largest accepted request body; contract requests are small.
const MAX_BODY_BYTES: usize = 1 << 20;

/// One live listener over one set of served sources.
pub struct ServeHandle {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl ServeHandle {
    /// The bound loopback address, for consumers and tests.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop accepting requests and join the workers.
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl Drop for ServeHandle {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// Bind the substrate on loopback and serve until shut down.
///
/// `port` zero asks the operating system for a free port; the bound
/// address is on the returned handle.
pub fn serve(sources: ServeSources, port: u16) -> Result<ServeHandle, HarnessError> {
    let server = Server::http(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .map_err(|error| HarnessError::Storage(format!("listener bind failed: {error}")))?;
    let addr = match server.server_addr().to_ip() {
        Some(addr) => addr,
        None => {
            return Err(HarnessError::Storage(
                "listener bound a non-ip address".to_string(),
            ))
        }
    };
    let server = Arc::new(server);
    let sources = Arc::new(sources);
    let stop = Arc::new(AtomicBool::new(false));

    let workers = (0..WORKER_THREADS)
        .map(|_| {
            let server = Arc::clone(&server);
            let sources = Arc::clone(&sources);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || worker_loop(&server, &sources, &stop))
        })
        .collect();

    Ok(ServeHandle {
        addr,
        stop,
        workers,
    })
}

fn worker_loop(server: &Server, sources: &ServeSources, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        let request = match server.recv_timeout(ACCEPT_TIMEOUT) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            // The listener socket is gone; nothing left to serve.
            Err(_) => break,
        };
        let mut request = request;
        let method = match request.method() {
            Method::Get => "GET",
            Method::Post => "POST",
            other => {
                let response = crate::serve::routes::RouteResponse {
                    status: 405,
                    body: format!("{{\"error\":\"method {other} not served\"}}").into_bytes(),
                };
                send(request, response);
                continue;
            }
        };
        let path = request.url().to_string();
        let mut body = Vec::new();
        let read = request
            .as_reader()
            .take(MAX_BODY_BYTES as u64 + 1)
            .read_to_end(&mut body);
        let response = match read {
            Ok(_) if body.len() > MAX_BODY_BYTES => RouteResponse {
                status: 413,
                body: b"{\"error\":\"request body too large\"}".to_vec(),
            },
            Ok(_) => dispatch(sources, method, &path, &body),
            Err(error) => RouteResponse {
                status: 400,
                body: format!("{{\"error\":\"body read failed: {error}\"}}").into_bytes(),
            },
        };
        send(request, response);
    }
}

/// Write one response; a consumer that hung up is its own problem.
fn send(request: tiny_http::Request, response: RouteResponse) {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static header is valid");
    let _ = request.respond(
        Response::from_data(response.body)
            .with_status_code(response.status)
            .with_header(header),
    );
}

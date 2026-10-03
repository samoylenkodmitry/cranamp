//! Desktop-only loopback transport. The editor commands remain shared with mobile/web.
use super::{mcp::dispatch, note, SharedDocument};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::JoinHandle,
    time::Duration,
};

pub const ADDRESS: &str = "127.0.0.1:18765";
pub const ENDPOINT: &str = "http://127.0.0.1:18765/mcp";
const TIMEOUT: Duration = Duration::from_secs(5);
const HEADER_LIMIT: usize = 16 * 1024;

#[derive(Clone, Default)]
pub struct Status {
    pub running: bool,
    pub stopping: bool,
    pub testing: bool,
    pub message: String,
}
#[derive(Default)]
struct Control {
    server: Option<Server>,
    status: Status,
    generation: u64,
}
static CONTROL: OnceLock<Mutex<Control>> = OnceLock::new();
fn control() -> &'static Mutex<Control> {
    CONTROL.get_or_init(Mutex::default)
}

struct Server {
    document: Arc<Mutex<SharedDocument>>,
    cancelled: Arc<AtomicBool>,
    connection: Arc<Mutex<Option<TcpStream>>>,
    worker: Option<JoinHandle<()>>,
}
impl Server {
    fn bind(listener: TcpListener, shared: SharedDocument) -> Result<Self> {
        anyhow::ensure!(
            listener.local_addr()?.ip().is_loopback(),
            "MCP requires a loopback address"
        );
        listener.set_nonblocking(true)?;
        let document = Arc::new(Mutex::new(shared));
        let cancelled = Arc::new(AtomicBool::new(false));
        let connection = Arc::new(Mutex::new(None));
        let (doc, cancel, current) = (document.clone(), cancelled.clone(), connection.clone());
        let worker = std::thread::Builder::new()
            .name("studio-mcp".into())
            .spawn(move || {
                while !cancel.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            // Accepted sockets inherit O_NONBLOCK on macOS.
                            if stream.set_nonblocking(false).is_err() {
                                continue;
                            }
                            let mut active = current.lock().unwrap();
                            if cancel.load(Ordering::Acquire) {
                                break;
                            }
                            *active = stream.try_clone().ok();
                            drop(active);
                            let _ = stream.set_read_timeout(Some(TIMEOUT));
                            let _ = stream.set_write_timeout(Some(TIMEOUT));
                            let shared = doc.lock().unwrap().clone();
                            let _ = serve(&mut stream, &shared);
                            current.lock().unwrap().take();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::park_timeout(Duration::from_millis(20));
                        }
                        Err(_) => break,
                    }
                }
            })?;
        Ok(Self {
            document,
            cancelled,
            connection,
            worker: Some(worker),
        })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(stream) = self.connection.lock().unwrap().take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

pub fn status() -> Status {
    let mut state = control().lock().unwrap();
    if state
        .server
        .as_ref()
        .is_some_and(|s| s.worker.as_ref().is_some_and(JoinHandle::is_finished))
    {
        state.server.take();
        state.status.running = false;
        state.status.message = "Server stopped unexpectedly. Try Start server.".into();
    }
    state.status.clone()
}
pub fn start(shared: SharedDocument) -> Result<()> {
    let result = {
        let mut state = control().lock().unwrap();
        anyhow::ensure!(!state.status.stopping, "Wait for the server to stop");
        if let Some(server) = &state.server {
            *server.document.lock().unwrap() = shared;
            return Ok(());
        }
        let result = TcpListener::bind(ADDRESS)
            .context("Cannot start MCP: port 18765 is already in use or unavailable")
            .and_then(|listener| Server::bind(listener, shared.clone()));
        state.generation += 1;
        match result {
            Ok(server) => {
                state.server = Some(server);
                state.status = Status {
                    running: true,
                    message: "Ready for a local agent. Use Test connection to verify.".into(),
                    ..Status::default()
                };
                Ok(())
            }
            Err(error) => {
                state.status = Status {
                    message: format!("{error:#}"),
                    ..Status::default()
                };
                Err(error)
            }
        }
    };
    note(&shared, status().message);
    result
}
pub fn stop_for(shared: &SharedDocument) {
    let server = {
        let mut state = control().lock().unwrap();
        if !state
            .server
            .as_ref()
            .is_some_and(|s| *s.document.lock().unwrap() == *shared)
        {
            return;
        }
        state.generation += 1;
        state.status = Status {
            stopping: true,
            message: "Stopping local connections…".into(),
            ..Status::default()
        };
        state.server.take()
    };
    note(shared, "Stopping agent server…".into());
    let shared = shared.clone();
    // An in-flight drawing/capture request may need the UI thread to finish.
    // Joining here on the UI thread would deadlock that request.
    std::thread::spawn(move || {
        drop(server);
        control().lock().unwrap().status = Status {
            message: "Stopped. Local agents cannot connect.".into(),
            ..Status::default()
        };
        note(&shared, "Agent server stopped".into());
    });
}

pub fn test_connection(shared: SharedDocument) {
    let generation = {
        let mut state = control().lock().unwrap();
        if !state.status.running || state.status.testing {
            return;
        }
        state.status.testing = true;
        state.status.message = "Testing the local MCP connection…".into();
        state.generation
    };
    note(&shared, "Testing agent connection…".into());
    std::thread::spawn(move || {
        let result = probe(ADDRESS.parse().expect("fixed loopback address"));
        let message = match result {
            Ok((version, count)) => {
                format!("Connection OK\nCranamp {version} · {count} tools available")
            }
            Err(error) => format!("Connection failed: {error:#}"),
        };
        let mut state = control().lock().unwrap();
        if state.generation != generation {
            return;
        }
        state.status.testing = false;
        state.status.message = message.clone();
        drop(state);
        note(&shared, message.replace('\n', " · "));
    });
}
fn request(address: SocketAddr, body: &str) -> Result<String> {
    let mut stream = TcpStream::connect_timeout(&address, TIMEOUT)
        .context("Open Skin Studio and start its agent server")?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    write!(stream, "POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())?;
    let mut response = String::new();
    stream
        .take(16 * 1024 * 1024)
        .read_to_string(&mut response)?;
    let (header, body) = response
        .split_once("\r\n\r\n")
        .context("Invalid HTTP response")?;
    anyhow::ensure!(
        header.starts_with("HTTP/1.1 200 ") || header.starts_with("HTTP/1.1 202 "),
        "MCP request rejected"
    );
    Ok(body.to_owned())
}
fn probe(address: SocketAddr) -> Result<(String, usize)> {
    let initialize: Value = serde_json::from_str(&request(
        address,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}).to_string(),
    )?)?;
    anyhow::ensure!(
        initialize["result"]["serverInfo"]["name"] == "cranamp-skin-studio",
        "This endpoint is not Cranamp Skin Studio"
    );
    let version = initialize["result"]["serverInfo"]["version"]
        .as_str()
        .context("Missing server version")?
        .to_owned();
    let tools: Value = serde_json::from_str(&request(
        address,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
    )?)?;
    let tools = tools["result"]["tools"]
        .as_array()
        .context("Missing tool list")?;
    anyhow::ensure!(
        tools.iter().any(|tool| tool["name"] == "studio_canvas"),
        "Skin Studio canvas tool is missing"
    );
    Ok((version, tools.len()))
}
pub fn bridge() {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        match request(ADDRESS.parse().expect("fixed loopback address"), &line) {
            Ok(response) if !response.is_empty() => println!("{response}"),
            Ok(_) => {}
            Err(error) => {
                let id = serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| value.get("id").cloned());
                if let Some(id) = id {
                    println!(
                        "{}",
                        json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32000, "message":format!("{error:#}")}})
                    );
                } else {
                    eprintln!("{error:#}");
                }
            }
        }
    }
}

fn serve(stream: &mut TcpStream, shared: &SharedDocument) -> Result<()> {
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    let mut consumed = (&mut reader)
        .take(HEADER_LIMIT as u64)
        .read_line(&mut line)?;
    let mut valid = line == "POST /mcp HTTP/1.1\r\n";
    let mut length = None;
    let mut host = false;
    let local = stream.local_addr()?;
    loop {
        if !valid || consumed >= HEADER_LIMIT {
            break;
        }
        line.clear();
        consumed += (&mut reader)
            .take((HEADER_LIMIT - consumed) as u64)
            .read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if !line.ends_with("\r\n") {
            valid = false;
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            if length.is_some() {
                valid = false;
                break;
            }
            length = v.trim().parse::<usize>().ok();
            valid &= length.is_some();
        }
        if let Some(value) = lower.strip_prefix("host:") {
            valid &= !host
                && (value.trim() == local.to_string()
                    || value.trim() == format!("localhost:{}", local.port()));
            host = true;
        }
        if lower.starts_with("origin:") || lower.starts_with("transfer-encoding:") {
            valid = false;
        }
    }
    if !valid
        || !host
        || consumed >= HEADER_LIMIT
        || length.is_none_or(|length| length > 8 * 1024 * 1024)
    {
        stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let mut body = vec![0; length.unwrap_or(0)];
    reader.read_exact(&mut body)?;
    let request: Value = serde_json::from_slice(&body)?;
    if request.get("id").is_none() {
        stream.write_all(
            b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let response = dispatch(request, shared);
    let bytes = serde_json::to_vec(&response)?;
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len())?;
    stream.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/agent_server.rs"]
mod tests;

//! The designer's local server (specs/17-local-designer-app.md, Decision 30; in Rust since
//! spec 39 step 2c). It serves the designer from `tools/designer/` on 127.0.0.1 only and gives
//! it a small JSON API:
//!
//! ```text
//! GET  /api/health               repo path, and whether Godot is configured
//! GET  /api/maps                 [{name, modified, imported}]
//! GET  /api/maps/<name>          the saved export
//! PUT  /api/maps/<name>          save it, and write content/maps/<name>.json from it
//!                                -> {saved, import: {ran, ok, warnings, errors, output_path}}
//! POST /api/maps/<name>/view     open content/maps/<name>.json in the Godot map viewer
//! GET  /api/libraries/<library>  content/designer/<library>.json (null if absent); "terrain" is
//!                                content/terrain/terrain_library.json, in the designer's shape
//! PUT  /api/libraries/<library>  write it -> {changed, import}; "terrain" is written in the
//!                                schema, and refused (import.ok false) if it drops a terrain
//!                                or feature a saved map uses
//! GET  /api/registry             the tags, traits and statuses, each trait's targets, and
//!                                where each is used (Decision 128)
//! ```
//!
//! HTTP/1.1, one request a connection, standard library only.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::registry::read_registry;
use super::repo::Repo;

/// Bumped whenever the API gains or changes an endpoint; repo.js compares it with its own
/// REQUIRED_API, so a server started before an update says to restart it.
pub const API_VERSION: i64 = 6;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
/// The page's files under `tools/designer/`, by the path they are served at.
pub const STATIC_FILES: [(&str, &str, &str); 13] = [
    ("/", "index.html", "text/html; charset=utf-8"),
    ("/index.html", "index.html", "text/html; charset=utf-8"),
    ("/designer.css", "designer.css", "text/css; charset=utf-8"),
    ("/designer.js", "designer.js", "text/javascript; charset=utf-8"),
    ("/repo.js", "repo.js", "text/javascript; charset=utf-8"),
    ("/load_paths.js", "load_paths.js", "text/javascript; charset=utf-8"),
    ("/planner_tools.js", "planner_tools.js", "text/javascript; charset=utf-8"),
    ("/planner_draw.js", "planner_draw.js", "text/javascript; charset=utf-8"),
    ("/planner_examples.js", "planner_examples.js", "text/javascript; charset=utf-8"),
    ("/planner_subnodes.js", "planner_subnodes.js", "text/javascript; charset=utf-8"),
    ("/planner.js", "planner.js", "text/javascript; charset=utf-8"),
    ("/library.html", "library.html", "text/html; charset=utf-8"),
    ("/library.js", "library.js", "text/javascript; charset=utf-8"),
];

/// A request as the server reads it.
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// A response: its status, type and body.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

fn json_response(status: u16, value: &Value) -> Response {
    Response {
        status,
        content_type: "application/json; charset=utf-8",
        body: value.to_string().into_bytes(),
    }
}

fn error(status: u16, message: &str) -> Response {
    json_response(status, &json!({"error": message}))
}

/// The server: the repo it works on, where the page's files are, and one lock for writes.
pub struct Server {
    pub repo: Repo,
    pub static_dir: PathBuf,
    writing: Mutex<()>,
}

impl Server {
    pub fn new(repo: Repo, static_dir: PathBuf) -> Server {
        Server { repo, static_dir, writing: Mutex::new(()) }
    }

    /// Serves `listener` until the process ends, one thread a connection.
    pub fn serve(self: Arc<Self>, listener: TcpListener) {
        for stream in listener.incoming().flatten() {
            let server = Arc::clone(&self);
            std::thread::spawn(move || server.answer(stream));
        }
    }

    fn answer(&self, mut stream: TcpStream) {
        let response = match read_request(&mut stream) {
            Ok(request) => self.handle(&request),
            Err(why) => error(400, &why),
        };
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            response.status,
            reason(response.status),
            response.content_type,
            response.body.len()
        );
        let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&response.body));
    }

    /// The response to `request`.
    pub fn handle(&self, request: &Request) -> Response {
        if !is_local(request) {
            return error(403, "forbidden");
        }
        let path = request.path.split('?').next().unwrap_or_default();
        if request.method == "GET" {
            if let Some((_, file, content_type)) =
                STATIC_FILES.iter().find(|(at, _, _)| *at == path)
            {
                return match std::fs::read(self.static_dir.join(file)) {
                    Ok(body) => Response { status: 200, content_type, body },
                    Err(why) => error(404, &why.to_string()),
                };
            }
        }
        let parts: Vec<String> =
            path.split('/').filter(|part| !part.is_empty()).map(percent_decoded).collect();
        let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
        match self.api(&request.method, &parts, request) {
            Ok(Some(value)) => json_response(200, &value),
            Ok(None) => error(404, "not found"),
            Err(Failure::NotFound(why)) => error(404, &why),
            Err(Failure::Bad(why)) => error(400, &why),
        }
    }

    fn api(
        &self,
        method: &str,
        parts: &[&str],
        request: &Request,
    ) -> Result<Option<Value>, Failure> {
        let repo = &self.repo;
        let value = match (method, parts) {
            ("GET", ["api", "health"]) => json!({
                "repo": repo.root.display().to_string(),
                "godot": repo.godot,
                "godot_found": repo.godot.is_some(),
                "api_version": API_VERSION,
            }),
            ("GET", ["api", "registry"]) => read_registry(&repo.root),
            ("GET", ["api", "maps"]) => repo.list_maps(),
            ("POST", ["api", "maps", name, "view"]) => repo.view_map(name)?,
            (_, ["api", "maps", _, "view"]) => {
                return Err(Failure::Bad("use POST to open the viewer".into()))
            }
            ("GET", ["api", "maps", name]) => match repo.read_map(name)? {
                Some(found) => found,
                None => return Err(Failure::NotFound(format!("no map named {name:?}"))),
            },
            ("PUT", ["api", "maps", name]) => {
                let body = read_body(request)?;
                let _held = self.writing.lock();
                repo.save_map(name, &body)?
            }
            ("GET", ["api", "libraries", library]) => repo.read_library(library)?,
            ("PUT", ["api", "libraries", library]) => {
                let body = read_body(request)?;
                let _held = self.writing.lock();
                repo.save_library(library, &body)?
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}

/// Why a request failed: 404 or 400.
enum Failure {
    NotFound(String),
    Bad(String),
}

impl From<String> for Failure {
    fn from(why: String) -> Failure {
        Failure::Bad(why)
    }
}

/// Only this machine's own pages: no DNS rebinding (the Host) and no cross-site writes (the
/// Origin).
fn is_local(request: &Request) -> bool {
    let local = |host: &str| matches!(host, "127.0.0.1" | "localhost");
    let host = request.header("Host").unwrap_or_default();
    let host = host.rsplit_once(':').map_or(host, |(name, _)| name);
    let origin_host = request.header("Origin").map(|origin| {
        let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
        let authority = rest.split('/').next().unwrap_or_default();
        authority.rsplit_once(':').map_or(authority, |(name, _)| name).to_string()
    });
    local(host) && origin_host.as_deref().is_none_or(local)
}

fn read_body(request: &Request) -> Result<Value, Failure> {
    if !request.header("Content-Type").unwrap_or_default().contains("application/json") {
        return Err(Failure::Bad("Content-Type must be application/json".into()));
    }
    if request.body.is_empty() || request.body.len() > MAX_BODY_BYTES {
        return Err(Failure::Bad(format!("body must be 1..{MAX_BODY_BYTES} bytes")));
    }
    serde_json::from_slice(&request.body)
        .map_err(|why| Failure::Bad(format!("body is not valid JSON: {why}")))
}

/// Reads one request: its line, headers and a Content-Length body.
pub fn read_request(stream: &mut impl Read) -> Result<Request, String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|why| why.to_string())?;
    let mut words = line.split_whitespace();
    let (Some(method), Some(path)) = (words.next(), words.next()) else {
        return Err("not an HTTP request".into());
    };
    let (method, path) = (method.to_string(), path.to_string());
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).map_err(|why| why.to_string())? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    let length = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(format!("body must be 1..{MAX_BODY_BYTES} bytes"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|why| why.to_string())?;
    Ok(Request { method, path, headers, body })
}

/// `%2F` and the like decoded, so a name can't smuggle a separator past the name check.
fn percent_decoded(part: &str) -> String {
    let bytes = part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |index: usize| bytes.get(index).and_then(|b| (*b as char).to_digit(16));
        match (bytes[at], hex(at + 1), hex(at + 2)) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                at += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        _ => "Not Found",
    }
}

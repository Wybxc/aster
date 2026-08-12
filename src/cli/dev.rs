use std::net::{IpAddr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{Context, Result};
use aster::engine::route::RoutePath;
use aster::{BuildOutcome, BuildSession};
use futures_util::stream;
use poem::error::ResponseError;
use poem::http::{HeaderValue, StatusCode, header};
use poem::listener::TcpAcceptor;
use poem::web::sse::{Event, SSE};
use poem::web::{Data, Path as RequestPath, Query, StaticFileRequest};
use poem::{Endpoint, EndpointExt, IntoResponse, Response, Route, Server, get, handler};
use scopeguard::{ScopeGuard, guard};
use serde::Deserialize;
use tokio::sync::{RwLock, oneshot, watch};

use crate::cli::watch::Watcher;
use crate::cli::{resolve_project, telemetry};

const LIVE_RELOAD_SCRIPT_PATH: &str = "/_aster/live-reload.js";
const LIVE_RELOAD_EVENTS_PATH: &str = "/_aster/events";
const LIVE_RELOAD_SCRIPT: &str = r#"(() => {
  const version = new URL(document.currentScript.src).searchParams.get("v") || "0";
  const events = new EventSource(`/_aster/events?since=${encodeURIComponent(version)}`);
  events.addEventListener("reload", () => location.reload());
})();
"#;

pub fn run(project_dir: Option<PathBuf>, host: IpAddr, port: u16) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let mut session = BuildSession::new(project.clone());
    let mut watcher = Watcher::new().context("failed to initialize file watcher")?;
    let server = DevServer::start(SocketAddr::new(host, port))?;
    tracing::info!(
        address = %format_args!("http://{}/", server.address()),
        "serving project at"
    );

    loop {
        match server.build(&mut session) {
            Ok(outcome) => telemetry::report_build(&outcome),
            Err(error) => tracing::error!(
                error = %format_args!("{error:#}"),
                "build failed: {error:#}"
            ),
        }

        watcher
            .replace(session.dependencies())
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .context("failed while watching project inputs")?;
        tracing::info!(reason = "change detected", "rebuilding after a change");
    }
}

#[derive(Clone, Default)]
struct SiteState {
    output_dir: Option<PathBuf>,
    revision: u64,
}

#[derive(Clone)]
struct ServerState {
    site: watch::Sender<SiteState>,
    output_access: Arc<RwLock<()>>,
}

type ServerResources = (oneshot::Sender<()>, JoinHandle<()>);
type ServerGuard = ScopeGuard<ServerResources, fn(ServerResources)>;

struct DevServer {
    address: SocketAddr,
    state: ServerState,
    _server: ServerGuard,
}

impl DevServer {
    fn start(address: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(address)
            .with_context(|| format!("failed to bind development server to {address}"))?;
        listener
            .set_nonblocking(true)
            .context("failed to configure development server listener")?;
        let address = listener
            .local_addr()
            .context("failed to read development server address")?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to initialize development server runtime")?;
        let (site, _) = watch::channel(SiteState::default());
        let state = ServerState {
            site,
            output_access: Arc::new(RwLock::new(())),
        };
        let app = application(state.clone());
        let (shutdown, stopped) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("aster-dev-server".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let acceptor = match TcpAcceptor::from_std(listener) {
                        Ok(acceptor) => acceptor,
                        Err(error) => {
                            tracing::error!(
                                error = %error,
                                "failed to initialize development server: {error}"
                            );
                            return;
                        }
                    };
                    let signal = async {
                        let _ = stopped.await;
                    };
                    if let Err(error) = Server::new_with_acceptor(acceptor)
                        .name("aster-dev")
                        .run_with_graceful_shutdown(app, signal, Some(Duration::from_millis(250)))
                        .await
                    {
                        tracing::error!(
                            error = %error,
                            "development server failed: {error}"
                        );
                    }
                });
            })
            .context("failed to start development server thread")?;

        Ok(Self {
            address,
            state,
            _server: guard((shutdown, thread), |(shutdown, thread)| {
                let _ = shutdown.send(());
                let _ = thread.join();
            }),
        })
    }

    fn address(&self) -> SocketAddr {
        self.address
    }

    fn build(&self, session: &mut BuildSession) -> Result<BuildOutcome> {
        let _output = self.state.output_access.blocking_write();
        let outcome = session.build()?;
        let revision = self.state.site.borrow().revision.saturating_add(1);
        self.state.site.send_replace(SiteState {
            output_dir: Some(outcome.output_dir.clone()),
            revision,
        });
        Ok(outcome)
    }
}

fn application(state: ServerState) -> impl Endpoint {
    Route::new()
        .at(LIVE_RELOAD_SCRIPT_PATH, get(live_reload_script))
        .at(LIVE_RELOAD_EVENTS_PATH, get(reload_events))
        .at("/*path", get(serve_site))
        .data(state)
}

#[handler]
fn live_reload_script() -> Response {
    bytes_response(
        StatusCode::OK,
        "text/javascript; charset=utf-8",
        LIVE_RELOAD_SCRIPT.as_bytes().to_vec(),
    )
}

#[derive(Deserialize)]
struct ReloadQuery {
    #[serde(default)]
    since: u64,
}

#[handler]
fn reload_events(
    Query(ReloadQuery { since }): Query<ReloadQuery>,
    Data(state): Data<&ServerState>,
) -> SSE {
    let receiver = state.site.subscribe();
    let events = stream::unfold((receiver, since), |(mut receiver, seen)| async move {
        loop {
            let revision = receiver.borrow().revision;
            if revision > seen {
                let event = Event::message(revision.to_string())
                    .id(revision.to_string())
                    .event_type("reload");
                return Some((event, (receiver, revision)));
            }
            if receiver.changed().await.is_err() {
                return None;
            }
        }
    });
    SSE::new(events).keep_alive(Duration::from_secs(15))
}

#[handler]
async fn serve_site(
    RequestPath(path): RequestPath<String>,
    static_request: StaticFileRequest,
    Data(state): Data<&ServerState>,
) -> Response {
    let path = if path.is_empty() {
        "index.html".to_owned()
    } else if path.ends_with('/') {
        format!("{path}index.html")
    } else {
        path
    };
    let route = match RoutePath::new(path) {
        Ok(route) => route,
        Err(_) => return status_page(StatusCode::BAD_REQUEST, "Invalid request path", 0),
    };
    serve_file(route, static_request, state).await
}

async fn serve_file(
    route: RoutePath,
    static_request: StaticFileRequest,
    state: &ServerState,
) -> Response {
    let _output = state.output_access.read().await;
    let site = state.site.borrow().clone();
    let Some(output_dir) = site.output_dir.as_deref() else {
        return status_page(
            StatusCode::SERVICE_UNAVAILABLE,
            "Waiting for a successful build",
            site.revision,
        );
    };
    let (path, not_found) = match route.as_virtual_path().realize(output_dir) {
        Ok(path) if path.is_file() => (path, false),
        _ => {
            let path = output_dir.join("404.html");
            if !path.is_file() {
                return status_page(StatusCode::NOT_FOUND, "Page not found", site.revision);
            }
            (path, true)
        }
    };

    let mut content = match tokio::fs::read(&path).await {
        Ok(content) => content,
        Err(error) => {
            tracing::error!(
                path = %path.display(),
                error = %error,
                "failed to serve {}: {error}",
                path.display()
            );
            return status_page(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read requested file",
                site.revision,
            );
        }
    };
    let content_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    if content_type == "text/html" {
        content = inject_reload_script(content, site.revision);
    }

    let mut response = match static_request.create_response_from_data(content) {
        Ok(response) => response.with_content_type(content_type).into_response(),
        Err(error) => error.as_response(),
    };
    if not_found {
        response.set_status(StatusCode::NOT_FOUND);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn inject_reload_script(content: Vec<u8>, revision: u64) -> Vec<u8> {
    let mut html = match String::from_utf8(content) {
        Ok(html) => html,
        Err(error) => return error.into_bytes(),
    };
    let script = format!(r#"<script src="{LIVE_RELOAD_SCRIPT_PATH}?v={revision}"></script>"#);
    let position = find_ascii_case_insensitive(&html, "</body>").unwrap_or(html.len());
    html.insert_str(position, &script);
    html.into_bytes()
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .rposition(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn status_page(status: StatusCode, message: &str, revision: u64) -> Response {
    let content = format!(
        "<!doctype html><html><head><title>{status}</title></head><body><h1>{message}</h1></body></html>"
    );
    bytes_response(
        status,
        "text/html; charset=utf-8",
        inject_reload_script(content.into_bytes(), revision),
    )
}

fn bytes_response(status: StatusCode, content_type: &'static str, content: Vec<u8>) -> Response {
    let length = content.len().to_string();
    Response::builder()
        .status(status)
        .content_type(content_type)
        .header(header::CONTENT_LENGTH, length)
        .header(header::CACHE_CONTROL, "no-store")
        .body(content)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use aster::Project;
    use poem::Request;
    use poem::http::Method;
    use tokio::io::AsyncReadExt;

    use super::*;

    fn state(output_dir: Option<PathBuf>, revision: u64) -> ServerState {
        let (site, _) = watch::channel(SiteState {
            output_dir,
            revision,
        });
        ServerState {
            site,
            output_access: Arc::new(RwLock::new(())),
        }
    }

    #[test]
    fn serves_explicit_directory_indexes_and_rejects_traversal() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("about")).unwrap();
        std::fs::write(
            temp.path().join("about/index.html"),
            "<!doctype html><html><body>About</body></html>",
        )
        .unwrap();
        std::fs::write(temp.path().join("foo"), "Exact file").unwrap();
        let app = application(state(Some(temp.path().to_owned()), 7));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let response = app
                .get_response(Request::builder().uri("/about".parse().unwrap()).finish())
                .await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(
                response
                    .into_body()
                    .into_string()
                    .await
                    .unwrap()
                    .contains("Page not found")
            );

            std::fs::write(
                temp.path().join("404.html"),
                "<!doctype html><html><body>Custom not found</body></html>",
            )
            .unwrap();

            let response = app
                .get_response(Request::builder().uri("/foo".parse().unwrap()).finish())
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.into_body().into_string().await.unwrap(),
                "Exact file"
            );

            let response = app
                .get_response(Request::builder().uri("/foo/".parse().unwrap()).finish())
                .await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let html = response.into_body().into_string().await.unwrap();
            assert!(html.contains("Custom not found"));
            assert!(html.contains("/_aster/live-reload.js?v=7"));

            let response = app
                .get_response(Request::builder().uri("/about/".parse().unwrap()).finish())
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            let html = response.into_body().into_string().await.unwrap();
            assert!(html.contains("About"));
            assert!(html.contains("/_aster/live-reload.js?v=7"));

            let response = app
                .get_response(
                    Request::builder()
                        .uri("/about/index.html".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            let html = response.into_body().into_string().await.unwrap();
            assert!(html.contains("About"));

            let response = app
                .get_response(
                    Request::builder()
                        .uri("/about.html".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let html = response.into_body().into_string().await.unwrap();
            assert!(html.contains("Custom not found"));

            let response = app
                .get_response(
                    Request::builder()
                        .uri("/404.html".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            let html = response.into_body().into_string().await.unwrap();
            assert!(html.contains("Custom not found"));

            let response = app
                .get_response(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/about/".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(response.into_body().into_bytes().await.unwrap().is_empty());

            let response = app
                .get_response(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/missing".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(response.into_body().into_bytes().await.unwrap().is_empty());

            let response = app
                .get_response(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/about".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

            let response = app
                .get_response(
                    Request::builder()
                        .uri("/%2e%2e/secret".parse().unwrap())
                        .finish(),
                )
                .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        });
    }

    fn get(server: &DevServer, path: &str) -> String {
        let mut connection = TcpStream::connect(server.address()).unwrap();
        write!(
            connection,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn server_publishes_successful_builds_over_http() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("pages")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let page = root.join("pages/index.typ");
        std::fs::write(&page, "#html.elem(\"p\")[First]").unwrap();
        let mut session = BuildSession::new(Project::open(root.to_owned()).unwrap());
        let server = DevServer::start("127.0.0.1:0".parse().unwrap()).unwrap();

        let outcome = server.build(&mut session).unwrap();
        assert_eq!(outcome.output_dir, root.join("dist"));
        let response = get(&server, "/");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("First"));
        assert!(response.contains("/_aster/live-reload.js?v=1"));

        std::fs::write(&page, "#let broken =").unwrap();
        assert!(server.build(&mut session).is_err());
        let response = get(&server, "/");
        assert!(response.contains("First"));
        assert!(response.contains("/_aster/live-reload.js?v=1"));

        std::fs::write(&page, "#html.elem(\"p\")[Second]").unwrap();
        server.build(&mut session).unwrap();
        let response = get(&server, "/");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("Second"));
        assert!(response.contains("/_aster/live-reload.js?v=2"));
    }

    #[test]
    fn reload_stream_reports_a_missed_revision() {
        let state = state(None, 3);
        let app = application(state);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let response = app
                .get_response(
                    Request::builder()
                        .uri("/_aster/events?since=2".parse().unwrap())
                        .finish(),
                )
                .await;
            let mut body = response.into_body().into_async_read();
            let mut buffer = [0; 64];
            let length = tokio::time::timeout(Duration::from_secs(1), body.read(&mut buffer))
                .await
                .unwrap()
                .unwrap();
            let event = std::str::from_utf8(&buffer[..length]).unwrap();
            assert!(event.contains("event: reload"));
            assert!(event.contains("data: 3"));
        });
    }
}

//! Reachability probe of the Nakama `/healthcheck` endpoint, used by the non-production status panel.

use crossbeam_channel::Receiver;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const TCP_TIMEOUT: Duration = crate::CONNECT_TIMEOUT;

/// Outcome of one `GET /healthcheck`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthResult {
    /// The server answered; `ok` is true for a 2xx status.
    Responded {
        ok: bool,
        status: u16,
        latency_ms: u64,
    },
    /// No answer (DNS, TLS, refused, timeout...).
    Failed { error: String, latency_ms: u64 },
}

/// `http(s)://host[:port]/healthcheck`, with the same TLS rule as [`crate::endpoint`].
pub fn healthcheck_url(host: &str, port: u16) -> String {
    format!("{}/healthcheck", crate::endpoint("http", host, port))
}

/// Joins an error with its `source()` chain ("a: b: c"), since transport errors keep the useful
/// cause (TLS certificate, DNS, ...) in the sources rather than in their own text.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(cause) = source {
        parts.push(cause.to_string());
        source = cause.source();
    }
    parts.join(": ")
}

/// Shortens a transport error to something that fits a small panel, keeping the root cause.
fn short_error(error: &reqwest::Error) -> String {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else {
        "request"
    };
    let chain = error_chain(error);
    // The chain mentions the public URL only; keep the tail, where the root cause is.
    let tail: String = chain
        .chars()
        .rev()
        .take(120)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{kind}: {tail}")
}

/// Runs a single probe on its own thread; the result arrives on the returned channel.
pub fn spawn_healthcheck(host: String, port: u16) -> Receiver<HealthResult> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let spawned = thread::Builder::new()
        .name("nakama-healthcheck".to_string())
        .spawn({
            let tx = tx.clone();
            move || {
                let _ = tx.send(probe_blocking(&host, port, &healthcheck_url(&host, port)));
            }
        });
    if let Err(e) = spawned {
        let _ = tx.send(HealthResult::Failed {
            error: e.to_string(),
            latency_ms: 0,
        });
    }
    rx
}

/// DNS and TCP stages run before the HTTP request so a failure says *where* it happened
/// ("dns: ...", "tcp: ...") instead of a bare timeout.
fn probe_network(host: &str, port: u16, elapsed: &dyn Fn() -> u64) -> Result<(), HealthResult> {
    let fail = |error: String| HealthResult::Failed {
        error,
        latency_ms: elapsed(),
    };
    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| fail(format!("dns: {e}")))?
        .collect();
    if addrs.is_empty() {
        return Err(fail("dns: no addresses".to_string()));
    }
    // Hosts behind Cloudflare list several addresses and one may be unreachable from this
    // network: the TCP stage passes as soon as any of them accepts a connection.
    let mut last = String::new();
    for addr in &addrs {
        match TcpStream::connect_timeout(addr, TCP_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(e) => last = format!("tcp {addr}: {e}"),
        }
    }
    Err(fail(last))
}

fn probe_blocking(host: &str, port: u16, url: &str) -> HealthResult {
    let started = Instant::now();
    let elapsed = || started.elapsed().as_millis() as u64;
    if let Err(result) = probe_network(host, port, &elapsed) {
        return result;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            return HealthResult::Failed {
                error: e.to_string(),
                latency_ms: 0,
            };
        }
    };
    runtime.block_on(async {
        let client = match reqwest::Client::builder()
            .connect_timeout(crate::CONNECT_TIMEOUT)
            .timeout(PROBE_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return HealthResult::Failed {
                    error: short_error(&e),
                    latency_ms: 0,
                };
            }
        };
        match client.get(url).send().await {
            Ok(resp) => HealthResult::Responded {
                ok: resp.status().is_success(),
                status: resp.status().as_u16(),
                latency_ms: elapsed(),
            },
            Err(e) => HealthResult::Failed {
                error: short_error(&e),
                latency_ms: elapsed(),
            },
        }
    })
}

#[cfg(test)]
mod tests {
    #[derive(Debug)]
    struct Outer(Inner);
    #[derive(Debug)]
    struct Inner;

    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "error sending request")
        }
    }
    impl std::fmt::Display for Inner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "invalid peer certificate")
        }
    }
    impl std::error::Error for Inner {}
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn error_chain_keeps_the_root_cause() {
        assert_eq!(
            error_chain(&Outer(Inner)),
            "error sending request: invalid peer certificate"
        );
    }

    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn url_follows_endpoint_scheme_rule() {
        assert_eq!(
            healthcheck_url("127.0.0.1", 7350),
            "http://127.0.0.1:7350/healthcheck"
        );
        assert_eq!(
            healthcheck_url("dev.example.com", 443),
            "https://dev.example.com/healthcheck"
        );
    }

    fn serve_once(status_line: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            // The TCP-stage probe connects first and sends nothing, then the HTTP request follows.
            while let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
            }
        });
        port
    }

    #[test]
    fn reports_ok_for_200() {
        let port = serve_once("200 OK");
        let rx = spawn_healthcheck("127.0.0.1".to_string(), port);
        match rx.recv_timeout(Duration::from_secs(10)).unwrap() {
            HealthResult::Responded { ok, status, .. } => {
                assert!(ok);
                assert_eq!(status, 200);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reports_not_ok_for_503() {
        let port = serve_once("503 Service Unavailable");
        let rx = spawn_healthcheck("127.0.0.1".to_string(), port);
        match rx.recv_timeout(Duration::from_secs(10)).unwrap() {
            HealthResult::Responded { ok, status, .. } => {
                assert!(!ok);
                assert_eq!(status, 503);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reports_failure_when_nothing_listens() {
        // Bind then drop to get a port that is (almost certainly) closed.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let rx = spawn_healthcheck("127.0.0.1".to_string(), port);
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            HealthResult::Failed { .. }
        ));
    }
}

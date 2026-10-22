use std::time::Duration;

use async_net::TcpStream;
use futures_lite::future;

pub async fn probe_tcp(host: &str, port: u16, timeout: Duration) -> bool {
    let address = format!("{}:{}", host, port);
    let connect = async move { TcpStream::connect(&address).await.is_ok() };
    let deadline = async move {
        async_io::Timer::after(timeout).await;
        false
    };

    future::or(connect, deadline).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn listener() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let port = listener.local_addr().expect("local addr").port();
        (listener, port)
    }

    #[test]
    fn probe_tcp_succeeds_against_open_listener() {
        let (_listener, port) = listener();
        let result =
            futures_lite::future::block_on(probe_tcp("127.0.0.1", port, Duration::from_secs(1)));
        assert!(result);
    }

    #[test]
    fn probe_tcp_fails_against_closed_port() {
        let result =
            futures_lite::future::block_on(probe_tcp("127.0.0.1", 1, Duration::from_secs(1)));
        assert!(!result);
    }

    #[test]
    fn probe_tcp_respects_timeout() {
        // 203.0.113.0/24 is TEST-NET-3, reserved for documentation and never
        // routed, so connecting hangs and the timeout must win.
        let start = std::time::Instant::now();
        let result =
            futures_lite::future::block_on(probe_tcp("203.0.113.1", 80, Duration::from_millis(50)));
        assert!(!result && start.elapsed() < Duration::from_secs(2));
    }
}

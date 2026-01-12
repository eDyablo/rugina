use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub(super) struct Config {
    endpoint: SocketAddr,
}

type Port = u16;

const OS_ASSIGNED_PORT: Port = 0;

impl Config {
    pub(super) fn from_env() -> Self {
        let port = std::env::var("SERVICE_PORT")
            .ok()
            .and_then(|v| v.parse::<Port>().ok())
            .unwrap_or(OS_ASSIGNED_PORT);

        let host = std::env::var("SERVICE_HOST")
            .ok()
            .and_then(|v| v.parse::<IpAddr>().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        Self {
            endpoint: SocketAddr::new(host, port),
        }
    }

    pub(super) fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }
}

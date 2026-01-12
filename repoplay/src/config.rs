use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;

pub(super) struct Config {
    endpoint: SocketAddr,
    storage_path: PathBuf,
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

        let storage_path = std::env::var("SERVICE_STORAGE_PATH")
            .ok()
            .and_then(|v| v.parse::<PathBuf>().ok())
            .unwrap_or(NamedTempFile::new().unwrap().path().to_path_buf());

        Self {
            endpoint: SocketAddr::new(host, port),
            storage_path,
        }
    }

    pub(super) fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }

    pub(super) fn storage_path(&self) -> &Path {
        self.storage_path.as_path()
    }
}

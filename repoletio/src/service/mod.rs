use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use repolet::{entity::Identifiable, repository::Repository};

pub struct Config {
    endpoint: SocketAddr,
}

impl Config {
    pub fn new() -> Self {
        Config {
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
        }
    }

    pub fn endpoint(mut self, endpoint: SocketAddr) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn repository<T, R>(self, repository: R) -> RepositoryConfig<T, R>
    where
        T: Identifiable + Send,
        R: Repository<T>,
    {
        RepositoryConfig {
            _marker: PhantomData,
            repository,
        }
    }
}

pub struct RepositoryConfig<T, R>
where
    T: Identifiable + Send,
    R: Repository<T>,
{
    _marker: PhantomData<T>,
    repository: R,
}

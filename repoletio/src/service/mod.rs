use std::net::SocketAddr;

use repolet::{entity::Identifiable, repository::Repository};

pub struct Set<T>(pub T);

pub struct NotSet;

pub struct Config<Endpoint = NotSet, Repository = NotSet> {
    endpont: Endpoint,
    repository: Repository,
}

impl Config<NotSet, NotSet> {
    pub fn new() -> Self {
        Self {
            endpont: NotSet,
            repository: NotSet,
        }
    }
}

impl<Endpoint, RepositoryT> Config<Endpoint, RepositoryT> {
    pub fn endpoint(
        self,
        endpoint: impl Into<SocketAddr>,
    ) -> Config<Set<SocketAddr>, RepositoryT> {
        Config {
            endpont: Set(endpoint.into()),
            repository: self.repository,
        }
    }

    pub fn repository<R>(self, repository: R) -> Config<Endpoint, Set<R>>
    where
        R: Repository,
    {
        Config {
            endpont: self.endpont,
            repository: Set(repository),
        }
    }
}

impl<Endpoint, RepositoryT> Config<Set<Endpoint>, Set<RepositoryT>>
where
    RepositoryT: Repository,
{
    pub fn build(self) -> Service<RepositoryT>
    where
        RepositoryT: Repository,
    {
        Service {
            repository: self.repository.0,
        }
    }
}

pub struct Service<R>
where
    R: Repository,
{
    repository: R,
}

impl<R> Service<R>
where
    R: Repository,
{
    pub async fn add(
        &mut self,
        item: R::Item,
    ) -> Result<
        <<R as Repository>::Item as Identifiable>::Id,
        <R as Repository>::Error,
    > {
        self.repository.put(item).await
    }
}

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
};

use crate::{
    entity::{Generational, GenerationalId, Identifiable},
    repository,
};

#[derive(Debug, thiserror::Error)]
pub enum Error<Id, Index> {
    #[error("entity with id {0} is deleted")]
    DeletedEntity(Id),

    #[error("entity with id {0} not found")]
    EntityNotFound(Id),

    #[error("{0} is invalid id")]
    InvalidId(Id),

    #[error("{0} is invalid index")]
    InvalidIndex(Index),

    #[error("failed I/O")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize or deserialize")]
    Serialization(#[from] serde_json::Error),
}

pub trait IdIndexMap {
    type Id;
    type Index;

    fn id_to_index(id: Self::Id) -> Option<Self::Index>;
    fn index_to_id(index: Self::Index) -> Option<Self::Id>;
}

struct Slot<T> {
    habitat: Option<T>,
    generation: u32,
}

#[async_trait]
pub trait Repository {
    type Item: Identifiable + Send + Sync;
    type Index: Copy;
    type IdIndex: IdIndexMap<Id = <Self::Item as Identifiable>::Id, Index = Self::Index>;
    type Slot;
    type Error;

    async fn delete(
        &mut self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<(), Self::Error>;

    async fn identify(&mut self) -> Result<(), Self::Error>;

    async fn get(
        &self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<Option<Self::Item>, Self::Error>;

    async fn put(
        &mut self,
        entity: Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error>;

    async fn update(
        &mut self,
        entity: &Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error>;

    fn visit<F, U>(&self, visitor: F) -> Result<usize, Self::Error>
    where
        F: for<'a> FnMut(&'a Self::Item) -> U;

    fn index_to_id(
        &self,
        index: Self::Index,
    ) -> Option<<Self::Item as Identifiable>::Id> {
        Self::IdIndex::index_to_id(index)
    }

    fn id_to_index(
        &self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Option<Self::Index> {
        Self::IdIndex::id_to_index(id)
    }
}

#[async_trait]
pub trait Persistable {
    type Error;

    async fn dump<W>(&self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: AsyncWrite + Unpin + Send;

    async fn load<R>(&mut self, reader: &mut R) -> Result<(), Self::Error>
    where
        R: AsyncRead + Unpin + Send;
}

pub struct VecIdIndex {}

impl IdIndexMap for VecIdIndex {
    type Id = Generational<usize>;
    type Index = usize;

    fn id_to_index(id: Self::Id) -> Option<usize> {
        Some(*id.inner())
    }

    fn index_to_id(index: usize) -> Option<Self::Id> {
        Some(Self::Id::new(index, 1))
    }
}

pub struct VecRepository<T>
where
    T: Identifiable + Send,
    T::Id: GenerationalId<usize>,
    Self: Repository,
{
    storage: Vec<Slot<T>>,
}

impl<T> VecRepository<T>
where
    T: Identifiable + Clone + Send,
    T::Id: GenerationalId<usize>,
    Self: Repository,
{
    pub fn new() -> Self {
        VecRepository {
            storage: Vec::new(),
        }
    }

    pub async fn from_slice(source: &[T]) -> Self {
        let storage = source
            .iter()
            .cloned()
            .map(|item| Slot {
                habitat: Some(item),
                generation: Default::default(),
            })
            .collect();
        let mut repository = VecRepository { storage };
        let _ = repository.identify().await;
        repository
    }
}

impl<T> Default for VecRepository<T>
where
    T: Identifiable + Clone + Send,
    T::Id: GenerationalId<usize>,
    Self: Repository,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> Repository for VecRepository<T>
where
    T: Identifiable<Id = Generational<usize>> + Clone + Send + Sync,
    T::Id: GenerationalId<usize> + Copy,
{
    type Item = T;
    type Index = usize;
    type IdIndex = VecIdIndex;
    type Slot = Option<T>;
    type Error = Error<T::Id, Self::Index>;

    async fn delete(&mut self, id: T::Id) -> Result<(), Self::Error> {
        let index = self.id_to_index(id).ok_or(Error::InvalidId(id))?;
        let slot = self
            .storage
            .get_mut(index)
            .ok_or(Error::EntityNotFound(id))?;
        *slot = Slot {
            habitat: None,
            generation: slot.generation,
        };
        Ok(())
    }

    async fn identify(&mut self) -> Result<(), Self::Error> {
        let iter = self.storage.iter_mut().enumerate();
        for (index, slot) in iter {
            let id = Self::IdIndex::index_to_id(index)
                .ok_or(Error::InvalidIndex(index))?;
            if let Some(item) = slot.habitat.as_mut() {
                item.set_id(id).await;
            }
        }
        Ok(())
    }

    async fn get(&self, id: T::Id) -> Result<Option<T>, Self::Error> {
        let index = self.id_to_index(id).ok_or(Error::InvalidId(id))?;
        let slot = self.storage.get(index).ok_or(Error::EntityNotFound(id))?;
        if slot.habitat.is_none() {
            return Err(Error::DeletedEntity(id));
        }
        Ok(slot.habitat.clone())
    }

    async fn put(&mut self, mut entity: T) -> Result<T::Id, Self::Error> {
        let index = self.storage.len();
        let id = self.index_to_id(index).ok_or(Error::InvalidIndex(index))?;
        entity.set_id(id).await;
        self.storage.push(Slot {
            habitat: Some(entity),
            generation: Default::default(),
        });
        Ok(id)
    }

    async fn update(&mut self, entity: &T) -> Result<T::Id, Self::Error> {
        let id = entity.get_id().await;
        let index = self.id_to_index(id).ok_or(Error::InvalidId(id))?;
        let slot = self
            .storage
            .get_mut(index)
            .ok_or(Error::EntityNotFound(id))?;
        slot.habitat = Some(entity.clone());
        Ok(id)
    }

    fn visit<F, U>(&self, mut visitor: F) -> Result<usize, Self::Error>
    where
        F: for<'a> FnMut(&'a Self::Item) -> U,
    {
        let visited_count = 0usize;
        for slot in self.storage.iter() {
            if let Some(object) = slot.habitat.as_ref() {
                visitor(object);
            }
        }
        Ok(visited_count)
    }
}

#[async_trait]
impl<T> Persistable for VecRepository<T>
where
    T: Identifiable<Id = Generational<usize>>
        + Clone
        + DeserializeOwned
        + Send
        + Serialize
        + Sync,
    T::Id: GenerationalId<usize> + Copy,
{
    type Error = Error<T::Id, usize>;

    async fn dump<W>(&self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let items =
            self.storage.iter().filter_map(|slot| slot.habitat.as_ref());
        for item in items {
            let data = serde_json::to_vec(item)?;
            writer.write_all(&data).await?;
            writer.write_all(&[0xA]).await?
        }
        Ok(())
    }

    async fn load<R>(&mut self, reader: &mut R) -> Result<(), Self::Error>
    where
        R: AsyncRead + Unpin + Send,
    {
        let mut line = String::new();
        let mut reader = BufReader::new(reader);
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            let item: T = serde_json::from_str(&line)?;
            self.storage.push(Slot {
                habitat: Some(item),
                generation: Default::default(),
            });
        }
        Ok(())
    }
}

#[async_trait]
pub trait SelfPersistable {
    type Error;

    async fn dump(&self) -> Result<(), Self::Error>;
    async fn load(&mut self) -> Result<(), Self::Error>;
}

pub struct AutosaveRepository<R>
where
    R: Repository + SelfPersistable,
{
    inner: R,
}

impl<R> AutosaveRepository<R>
where
    R: Repository + SelfPersistable,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

pub struct FilePersistentRepository<R>
where
    R: Repository + Persistable,
{
    file_path: PathBuf,
    inner: R,
}

impl<R> FilePersistentRepository<R>
where
    R: Repository
        + Persistable<
            Error = repository::Error<
                <<R as Repository>::Item as Identifiable>::Id,
                R::Index,
            >,
        >,
    Self: SelfPersistable<Error = <R as Persistable>::Error>,
{
    pub async fn new(
        path: &Path,
        inner: R,
    ) -> Result<Self, <R as Persistable>::Error> {
        let mut repository = Self {
            file_path: path.to_path_buf(),
            inner,
        };
        if let Err(err) = repository.load().await {
            match &err {
                Error::Io(io_error)
                    if io_error.kind() == std::io::ErrorKind::NotFound =>
                {
                    // Do nothing, file missing is fine
                }
                _ => return Err(err),
            }
        }
        Ok(repository)
    }
}

#[async_trait]
impl<R> SelfPersistable for FilePersistentRepository<R>
where
    R: Repository + Persistable<Error = <R as Repository>::Error> + Send + Sync,
    <R as Repository>::Error: From<std::io::Error>,
{
    type Error = <R as Repository>::Error;

    async fn dump(&self) -> Result<(), Self::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.file_path.clone())
            .await?;
        self.inner.dump(&mut file).await
    }

    async fn load(&mut self) -> Result<(), Self::Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(self.file_path.clone())
            .await?;
        self.inner.load(&mut file).await
    }
}

#[async_trait]
impl<R> Repository for FilePersistentRepository<R>
where
    R: Repository + Persistable + Send + Sync,
{
    type Item = R::Item;
    type Index = R::Index;
    type IdIndex = R::IdIndex;
    type Slot = R::Slot;
    type Error = <R as Repository>::Error;

    async fn delete(
        &mut self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }

    async fn identify(&mut self) -> Result<(), Self::Error> {
        self.inner.identify().await
    }

    async fn get(
        &self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.get(id).await
    }

    async fn put(
        &mut self,
        entity: Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error> {
        self.inner.put(entity).await
    }

    async fn update(
        &mut self,
        entity: &Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error> {
        self.inner.update(entity).await
    }

    fn visit<F, U>(&self, visitor: F) -> Result<usize, Self::Error>
    where
        F: for<'a> FnMut(&'a Self::Item) -> U,
    {
        self.inner.visit(visitor)
    }
}

#[async_trait]
impl<R> Repository for AutosaveRepository<R>
where
    R: Repository
        + SelfPersistable<Error = <R as Repository>::Error>
        + Send
        + Sync,
    <R as Repository>::Error: Send,
{
    type Item = R::Item;
    type Index = R::Index;
    type IdIndex = R::IdIndex;
    type Slot = R::Slot;
    type Error = <R as Repository>::Error;

    async fn delete(
        &mut self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<(), Self::Error> {
        let result = self.inner.delete(id).await;
        self.inner.dump().await?;
        result
    }

    async fn identify(&mut self) -> Result<(), Self::Error> {
        self.inner.identify().await
    }

    async fn get(
        &self,
        id: <Self::Item as Identifiable>::Id,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.get(id).await
    }

    async fn put(
        &mut self,
        entity: Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error> {
        let result = self.inner.put(entity).await;
        self.inner.dump().await?;
        result
    }

    async fn update(
        &mut self,
        entity: &Self::Item,
    ) -> Result<<Self::Item as Identifiable>::Id, Self::Error> {
        let result = self.inner.update(entity).await;
        self.inner.dump().await?;
        result
    }

    fn visit<F, U>(&self, visitor: F) -> Result<usize, Self::Error>
    where
        F: for<'a> FnMut(&'a Self::Item) -> U,
    {
        self.inner.visit(visitor)
    }
}

#[cfg(test)]
mod tests;

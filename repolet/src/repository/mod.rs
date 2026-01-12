use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

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
pub trait Repository<T>
where
    T: Identifiable + Send,
{
    type Index: Copy;
    type IdIndex: IdIndexMap<Id = <T as Identifiable>::Id, Index = Self::Index>;
    type Slot;
    type Error;

    async fn delete(&mut self, id: T::Id) -> Result<(), Self::Error>;

    async fn identify(&mut self) -> Result<(), Self::Error>;

    async fn get(&self, id: T::Id) -> Result<Option<T>, Self::Error>;

    async fn put(&mut self, entity: T) -> Result<T::Id, Self::Error>;

    async fn update(&mut self, entity: &T) -> Result<T::Id, Self::Error>;

    async fn visit<F>(&self, mut visitor: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnMut(&'a T) + Send;

    fn index_to_id(&self, index: Self::Index) -> Option<T::Id> {
        Self::IdIndex::index_to_id(index)
    }

    fn id_to_index(&self, id: T::Id) -> Option<Self::Index> {
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
    Self: Repository<T>,
{
    storage: Vec<Slot<T>>,
}

impl<T> VecRepository<T>
where
    T: Identifiable + Clone + Send,
    T::Id: GenerationalId<usize>,
    Self: Repository<T>,
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
    Self: Repository<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> Repository<T> for VecRepository<T>
where
    T: Identifiable<Id = Generational<usize>> + Clone + Send + Sync,
    T::Id: GenerationalId<usize> + Copy,
{
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

    async fn visit<F>(&self, mut visitor: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnMut(&'a T) + Send,
    {
        for slot in self.storage.iter() {
            if let Some(object) = slot.habitat.as_ref() {
                visitor(object)
            }
        }
        Ok(())
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

pub struct AutosaveRepository<T, R>
where
    T: Identifiable + Send,
    R: Repository<T> + SelfPersistable,
{
    _phantom: PhantomData<T>,
    inner: R,
}

impl<T, R> AutosaveRepository<T, R>
where
    T: Identifiable + Send,
    R: Repository<T> + SelfPersistable,
{
    pub fn new(inner: R) -> Self {
        Self {
            _phantom: PhantomData,
            inner,
        }
    }
}

pub struct FilePersistentRepository<T, R>
where
    T: Identifiable + Send,
    R: Repository<T> + Persistable,
{
    _phantom: PhantomData<T>,
    file_path: PathBuf,
    inner: R,
}

impl<T, R> FilePersistentRepository<T, R>
where
    T: Identifiable + Send,
    R: Repository<T> + Persistable<Error = repository::Error<T::Id, R::Index>>,
    Self: SelfPersistable<Error = <R as Persistable>::Error>,
{
    pub async fn new(
        path: &Path,
        inner: R,
    ) -> Result<Self, <R as Persistable>::Error> {
        let mut repository = Self {
            _phantom: PhantomData,
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
impl<T, R> SelfPersistable for FilePersistentRepository<T, R>
where
    T: Identifiable + Send + Sync,
    R: Repository<T>
        + Persistable<Error = <R as Repository<T>>::Error>
        + Send
        + Sync,
    <R as Repository<T>>::Error: From<std::io::Error>,
{
    type Error = <R as Repository<T>>::Error;

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
impl<T, R> Repository<T> for FilePersistentRepository<T, R>
where
    T: Identifiable + Send + Sync,
    R: Repository<T> + Persistable + Send + Sync,
{
    type Index = R::Index;
    type IdIndex = R::IdIndex;
    type Slot = R::Slot;
    type Error = <R as Repository<T>>::Error;

    async fn delete(&mut self, id: T::Id) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }

    async fn identify(&mut self) -> Result<(), Self::Error> {
        self.inner.identify().await
    }

    async fn get(&self, id: T::Id) -> Result<Option<T>, Self::Error> {
        self.inner.get(id).await
    }

    async fn put(&mut self, entity: T) -> Result<T::Id, Self::Error> {
        self.inner.put(entity).await
    }

    async fn update(&mut self, entity: &T) -> Result<T::Id, Self::Error> {
        self.inner.update(entity).await
    }

    async fn visit<F>(&self, visitor: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnMut(&'a T) + Send,
    {
        self.inner.visit(visitor).await
    }
}

#[async_trait]
impl<T, R> Repository<T> for AutosaveRepository<T, R>
where
    T: Identifiable + Send + Sync,
    R: Repository<T>
        + SelfPersistable<Error = <R as Repository<T>>::Error>
        + Send
        + Sync,
    <R as Repository<T>>::Error: Send,
{
    type Index = R::Index;
    type IdIndex = R::IdIndex;
    type Slot = R::Slot;
    type Error = <R as Repository<T>>::Error;

    async fn delete(&mut self, id: T::Id) -> Result<(), Self::Error> {
        let result = self.inner.delete(id).await;
        self.inner.dump().await?;
        result
    }

    async fn identify(&mut self) -> Result<(), Self::Error> {
        self.inner.identify().await
    }

    async fn get(&self, id: T::Id) -> Result<Option<T>, Self::Error> {
        self.inner.get(id).await
    }

    async fn put(&mut self, entity: T) -> Result<T::Id, Self::Error> {
        let result = self.inner.put(entity).await;
        self.inner.dump().await?;
        result
    }

    async fn update(&mut self, entity: &T) -> Result<T::Id, Self::Error> {
        let result = self.inner.update(entity).await;
        self.inner.dump().await?;
        result
    }

    async fn visit<F>(&self, visitor: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnMut(&'a T) + Send,
    {
        self.inner.visit(visitor).await
    }
}

#[cfg(test)]
mod tests;

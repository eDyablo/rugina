use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[async_trait]
pub trait Identifiable {
    type Id: Copy + Send;

    async fn get_id(&self) -> Self::Id;
    async fn set_id(&mut self, id: Self::Id);
}

pub trait GenerationalId<T> {
    fn generation(&self) -> u32;
}

#[async_trait]
impl<T> Identifiable for Arc<Mutex<T>>
where
    T: Identifiable + Send,
{
    type Id = T::Id;

    async fn get_id(&self) -> Self::Id {
        self.lock().await.get_id().await
    }

    async fn set_id(&mut self, id: Self::Id) {
        self.lock().await.set_id(id).await
    }
}

pub struct Generational<T> {
    inner: T,
    generation: u32,
}

impl<T> Generational<T> {
    pub fn new(inner: T, generation: u32) -> Self {
        Self { inner, generation }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl<T: Clone> Clone for Generational<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            generation: self.generation,
        }
    }
}

impl<T: Copy> Copy for Generational<T> {}

impl<T: Debug> Debug for Generational<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Generational")
            .field("inner", &self.inner)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<T: Default> Default for Generational<T> {
    fn default() -> Self {
        Self {
            inner: T::default(),
            generation: Default::default(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Generational<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match T::deserialize(deserializer) {
            Ok(inner) => Ok(Generational {
                inner,
                generation: Default::default(),
            }),
            Err(err) => Err(err),
        }
    }
}

impl<T: Display> Display for Generational<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(formatter)
    }
}

impl<T> From<T> for Generational<T> {
    fn from(inner: T) -> Self {
        Generational::new(inner, Default::default())
    }
}

impl<T> GenerationalId<T> for Generational<T> {
    fn generation(&self) -> u32 {
        self.generation
    }
}

impl<T: PartialEq> PartialEq for Generational<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.generation == other.generation
    }
}

impl<T: Serialize> Serialize for Generational<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

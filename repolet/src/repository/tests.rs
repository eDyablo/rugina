use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::entity::{Generational, Identifiable};

#[derive(Debug, Default, Clone)]
struct Entity {
    id: Generational<usize>,
}

#[async_trait]
impl Identifiable for Entity {
    type Id = Generational<usize>;

    async fn get_id(&self) -> Self::Id {
        self.id
    }

    async fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}

type SharedEntity = Arc<Mutex<Entity>>;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Payload {
    #[serde(skip)]
    id: Generational<usize>,
    data: Vec<u8>,
}

#[async_trait]
impl Identifiable for Payload {
    type Id = Generational<usize>;

    async fn get_id(&self) -> Self::Id {
        self.id
    }

    async fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}

impl From<&[u8]> for Payload {
    fn from(data: &[u8]) -> Self {
        Self {
            id: Default::default(),
            data: data.into(),
        }
    }
}

impl From<&str> for Payload {
    fn from(data: &str) -> Self {
        Self {
            id: Default::default(),
            data: data.into(),
        }
    }
}

mod vec_repository {
    use std::future;

    use tokio::io::{self, AsyncWriteExt};

    use crate::entity::Identifiable;

    use crate::repository::Persistable;
    use crate::repository::tests::{Payload, SharedEntity};
    use crate::repository::{
        {Error, Repository},
        {VecRepository, tests::Entity},
    };

    #[macro_export]
    macro_rules! assert_error_variant {
        ($result:expr, $variant:path, $id:expr) => {{
            let id = $id.into();
            let result = $result;
            assert!(
                matches!(result, Err($variant(eid)) if eid == id),
                "Expected {:?}({:?}), got {:?}",
                stringify!($variant),
                id,
                result
            );
        }};
    }

    #[tokio::test]
    async fn get_nonexistent_returns_error() {
        let repository = VecRepository::<Entity>::new();
        assert_error_variant!(
            repository.get(0.into()).await,
            Error::EntityNotFound,
            0
        );
        assert_error_variant!(
            repository.get(1.into()).await,
            Error::EntityNotFound,
            1
        );
    }

    #[tokio::test]
    async fn get_existing_returns_valid_entity() {
        let repository =
            VecRepository::from_slice(&[Entity::default(), Entity::default()])
                .await;
        assert!(matches!(
            repository.get(0.into()).await,
            Ok(Some(entity)) if *entity.id.inner() == 0
        ));
        assert!(matches!(
            repository.get(1.into()).await,
            Ok(Some(entity)) if *entity.id.inner() == 1
        ));
    }

    #[tokio::test]
    async fn put_set_entity_id() {
        let mut repository = VecRepository::new();
        let item = SharedEntity::new(Entity::default().into());
        assert_eq!(
            repository.put(item.clone()).await.unwrap().inner(),
            item.get_id().await.inner()
        );
        assert_eq!(
            repository.put(item.clone()).await.unwrap().inner(),
            item.get_id().await.inner()
        );
        assert_eq!(
            repository.put(item.clone()).await.unwrap().inner(),
            item.get_id().await.inner()
        );
    }

    #[tokio::test]
    async fn visit_visits_all_entities_in_order() {
        let repository = VecRepository::from_slice(&[
            ([1][..]).into(),
            ([2][..]).into(),
            ([3][..]).into(),
        ])
        .await;
        let mut data = Vec::<u8>::new();
        let count = repository
            .visit(|i: &Payload| {
                data.extend(&i.data);
                future::ready(())
            })
            .unwrap();
        assert_eq!(count, 3);
        assert_eq!(data, &[1, 2, 3]);
    }

    #[tokio::test]
    async fn get_deleted_entity_returns_error() {
        let mut repository = VecRepository::from_slice(&[
            Entity::default(),
            Entity::default(),
            Entity::default(),
        ])
        .await;
        repository.delete(1.into()).await.unwrap();
        assert_error_variant!(
            repository.get(1.into()).await,
            Error::DeletedEntity,
            1
        );
    }

    #[tokio::test]
    async fn delete_deleted_entity_returns_ok() {
        let mut repository =
            VecRepository::from_slice(&[Entity::default()]).await;
        repository.delete(0.into()).await.unwrap();
        assert!(matches!(repository.delete(0.into()).await, Ok(())));
    }

    #[tokio::test]
    async fn put_entity_returns_first_generation_id() {
        let mut repository = VecRepository::<Entity>::new();
        assert!(matches!(repository.put(Entity::default()).await,
            Ok(id) if id.generation() == 1));
    }

    #[tokio::test]
    async fn update_nonexistent_returns_error() {
        let mut repository = VecRepository::<Entity>::new();
        assert_error_variant!(
            repository.update(&Entity { id: 0.into() }).await,
            Error::EntityNotFound,
            0
        );
    }

    #[tokio::test]
    async fn update_existing_updates_entity_and_returns_its_id() {
        let mut repository =
            VecRepository::from_slice(&["existing".into()]).await;
        assert!(matches!(
            repository.update(&Payload { id: 0.into(), data: "updated".into() }).await,
            Ok(id) if *id.inner() == 0
        ));
        assert!(matches!(
            repository.get(0.into()).await,
            Ok(Some(payload)) if payload.data == b"updated"
        ));
    }

    #[tokio::test]
    async fn dump_and_load_serializes_and_deserializes_entities() {
        let src = VecRepository::<Payload>::from_slice(&[
            "first".into(),
            "second".into(),
            "third".into(),
        ])
        .await;
        let (mut reader, mut writer) = io::duplex(500);
        src.dump(&mut writer).await.unwrap();
        writer.shutdown().await.unwrap();

        let mut dst = VecRepository::<Payload>::new();
        dst.load(&mut reader).await.unwrap();
        assert!(
            matches!(dst.get(0.into()).await, Ok(Some(payload)) if payload.data == b"first")
        );
        assert!(
            matches!(dst.get(1.into()).await, Ok(Some(payload)) if payload.data == b"second")
        );
        assert!(
            matches!(dst.get(2.into()).await, Ok(Some(payload)) if payload.data == b"third")
        );
    }
}

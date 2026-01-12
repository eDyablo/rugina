#[cfg(test)]
mod repository {
    use std::path::Path;

    use async_trait::async_trait;
    use repolet::{
        entity::{Generational, Identifiable},
        repository::{Repository, VecRepository},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Default, Serialize, Deserialize)]
    struct Payload {
        #[serde(skip)]
        id: Generational<usize>,
        data: String,
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

    impl From<&str> for Payload {
        fn from(data: &str) -> Self {
            Self {
                id: Default::default(),
                data: data.into(),
            }
        }
    }

    mod file_persitent {
        use std::io;

        use repolet::repository::Error;
        use repolet::repository::FilePersistentRepository;
        use repolet::repository::SelfPersistable;

        use super::Path;
        use super::Payload;
        use super::Repository;
        use super::VecRepository;

        #[tokio::test]
        async fn can_be_created_for_non_existent_file() {
            assert!(
                FilePersistentRepository::new(
                    Path::new("non existent file"),
                    VecRepository::<Payload>::new(),
                )
                .await
                .is_ok()
            );
        }

        #[tokio::test]
        async fn can_be_created_from_empty_file() {
            let path = Path::new("tests/data/repository/empty.txt");
            assert!(path.exists());
            assert!(
                FilePersistentRepository::new(
                    Path::new("non existent file"),
                    VecRepository::<Payload>::new(),
                )
                .await
                .is_ok()
            );
        }

        #[tokio::test]
        async fn load_fails_for_non_existent_file() {
            let path = Path::new("non existing file");
            assert!(!path.exists());
            let mut repository = FilePersistentRepository::new(
                path,
                VecRepository::<Payload>::new(),
            )
            .await
            .unwrap();
            assert!(
                matches!(repository.load().await, Err(Error::Io(ref io_error)) if io_error.kind() == io::ErrorKind::NotFound)
            );
        }

        #[tokio::test]
        async fn load_loads_from_file() {
            let mut repository = FilePersistentRepository::new(
                Path::new("tests/data/repository/payload.txt"),
                VecRepository::<Payload>::new(),
            )
            .await
            .unwrap();
            repository.load().await.unwrap();
            assert!(
                matches!(repository.get(0.into()).await, Ok(Some(payload)) if payload.data == "first")
            );
            assert!(
                matches!(repository.get(1.into()).await, Ok(Some(payload)) if payload.data == "second")
            );
        }

        mod auto_save {
            use std::fs;
            use std::path::Path;

            use repolet::repository::AutosaveRepository;
            use repolet::repository::Repository;
            use tempfile::NamedTempFile;

            use super::FilePersistentRepository;
            use super::Payload;
            use super::VecRepository;

            #[tokio::test]
            async fn put_saves_to_file() {
                let temporary = NamedTempFile::new().unwrap();
                assert!(temporary.path().exists());
                assert_eq!(fs::metadata(temporary.path()).unwrap().len(), 0);
                let mut repository = AutosaveRepository::new(
                    FilePersistentRepository::new(
                        temporary.path(),
                        VecRepository::<Payload>::new(),
                    )
                    .await
                    .unwrap(),
                );
                repository.put("payload".into()).await.unwrap();
                assert!(fs::metadata(temporary.path()).unwrap().len() > 0);
            }

            #[tokio::test]
            async fn delete_saves_to_file() {
                let temporary = NamedTempFile::new().unwrap();
                assert!(temporary.path().exists());
                assert_eq!(fs::metadata(temporary.path()).unwrap().len(), 0);
                let mut repository = AutosaveRepository::new(
                    FilePersistentRepository::new(
                        temporary.path(),
                        VecRepository::<Payload>::new(),
                    )
                    .await
                    .unwrap(),
                );
                let id = repository.put("payload".into()).await.unwrap();
                repository.delete(id).await.unwrap();
                assert_eq!(fs::metadata(temporary.path()).unwrap().len(), 0);
            }

            #[tokio::test]
            async fn update_saves_to_file() {
                let source = Path::new("tests/data/repository/payload.txt");
                let temporary = NamedTempFile::new().unwrap();
                fs::copy(source, temporary.path()).unwrap();
                let mut repository = AutosaveRepository::new(
                    FilePersistentRepository::new(
                        temporary.path(),
                        VecRepository::<Payload>::new(),
                    )
                    .await
                    .unwrap(),
                );
                assert_eq!(
                    fs::metadata(source).unwrap().len(),
                    fs::metadata(temporary.path()).unwrap().len()
                );
                repository
                    .update(&Payload {
                        id: 0.into(),
                        data: "updated payload".into(),
                    })
                    .await
                    .unwrap();
                assert_ne!(
                    fs::metadata(source).unwrap().len(),
                    fs::metadata(temporary.path()).unwrap().len()
                );
            }
        }
    }
}

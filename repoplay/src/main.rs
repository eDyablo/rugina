mod config;
mod entity;


use repolet::repository::{
    AutosaveRepository, FilePersistentRepository, VecRepository,
};
use repoletio::service;

use crate::{config::Config, entity::Payload};

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let repository = AutosaveRepository::new(
        FilePersistentRepository::new(
            config.storage_path(),
            VecRepository::<Payload>::new(),
        )
        .await
        .unwrap(),
    );
    let mut service = service::Config::new()
        .repository(repository)
        .endpoint(config.endpoint())
        .build();
    service
        .add(Payload {
            id: 0.into(),
            _data: "hello".into(),
        })
        .await
        .unwrap();
}

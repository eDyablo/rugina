mod config;
mod entity;

use repolet::repository::VecRepository;
use repoletio::service;

use crate::{config::Config, entity::Payload};

pub fn main() {
    let config = Config::from_env();
    let _ = service::Config::new()
        .endpoint(config.endpoint())
        .repository(VecRepository::<Payload>::new());
}

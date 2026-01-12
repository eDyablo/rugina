use async_trait::async_trait;
use repolet::entity::{Generational, Identifiable};

#[derive(Clone)]
pub(super) struct Payload {
    id: Generational<usize>,
    _data: Vec<u8>,
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

use async_trait::async_trait;
use repolet::entity::{Generational, Identifiable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Payload {
    #[serde(skip)]
    pub id: Generational<usize>,
    pub _data: Vec<u8>,
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

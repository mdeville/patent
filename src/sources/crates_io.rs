//! crates.io source — `GET https://crates.io/api/v1/crates?q=`.

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

/// Searches the crates.io registry.
#[derive(Debug, Default, Clone)]
pub struct CratesIo {
    client: reqwest::Client,
}

impl CratesIo {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Source for CratesIo {
    fn id(&self) -> SourceId {
        SourceId::CratesIo
    }

    async fn search(&self, _query: &Query) -> Result<Vec<Match>> {
        let _ = &self.client;
        todo!("M1: query crates.io and map results into Match")
    }
}

//! npm source — `GET https://registry.npmjs.org/-/v1/search?text=`.

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

/// Searches the npm registry.
#[derive(Debug, Default, Clone)]
pub struct Npm {
    client: reqwest::Client,
}

impl Npm {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Source for Npm {
    fn id(&self) -> SourceId {
        SourceId::Npm
    }

    async fn search(&self, _query: &Query) -> Result<Vec<Match>> {
        let _ = &self.client;
        todo!("M2: query npm search and map results into Match")
    }
}

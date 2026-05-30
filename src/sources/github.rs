//! GitHub source — `GET /search/repositories`. Reads optional `GITHUB_TOKEN`
//! from the environment to raise the unauthenticated rate limit.

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

/// Searches GitHub repositories.
#[derive(Debug, Default, Clone)]
pub struct GitHub {
    client: reqwest::Client,
}

impl GitHub {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Source for GitHub {
    fn id(&self) -> SourceId {
        SourceId::GitHub
    }

    async fn search(&self, _query: &Query) -> Result<Vec<Match>> {
        let _ = &self.client;
        todo!("M2: query GitHub repo search (optional GITHUB_TOKEN)")
    }
}

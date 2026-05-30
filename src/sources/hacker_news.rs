//! Hacker News source — `GET https://hn.algolia.com/api/v1/search?query=`
//! (free, no key).

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

/// Searches Hacker News via the Algolia API.
#[derive(Debug, Default, Clone)]
pub struct HackerNews {
    client: reqwest::Client,
}

impl HackerNews {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Source for HackerNews {
    fn id(&self) -> SourceId {
        SourceId::HackerNews
    }

    async fn search(&self, _query: &Query) -> Result<Vec<Match>> {
        let _ = &self.client;
        todo!("M2: query HN Algolia search and map results into Match")
    }
}

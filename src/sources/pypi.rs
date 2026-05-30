//! PyPI source.
//!
//! Note: PyPI has no public search API (the XML-RPC search endpoint was
//! disabled). For v0 this either scrapes `https://pypi.org/search/?q=` or is
//! dropped — resolve during M2; do not block the other four sources on it.

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

/// Searches PyPI (scrape-based; see module note).
#[derive(Debug, Default, Clone)]
pub struct PyPI {
    client: reqwest::Client,
}

impl PyPI {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Source for PyPI {
    fn id(&self) -> SourceId {
        SourceId::PyPI
    }

    async fn search(&self, _query: &Query) -> Result<Vec<Match>> {
        let _ = &self.client;
        todo!("M2: scrape pypi.org/search (or drop for v0)")
    }
}

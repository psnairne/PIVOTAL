use crate::hgnc::enums::GeneQuery;
use crate::hgnc::error::HGNCError;
use crate::hgnc::json_schema::{GeneDoc, GeneResponse};
use crate::hgnc::traits::HGNCData;
use ratelimit::Ratelimiter;
use reqwest::blocking::Client;
use std::fmt::{Debug, Formatter};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;

static HGNC_RATE_LIMITER: OnceLock<Ratelimiter> = OnceLock::new();

fn hgnc_rate_limiter() -> &'static Ratelimiter {
    HGNC_RATE_LIMITER.get_or_init(|| {
        Ratelimiter::builder(10, Duration::from_millis(1100))
            .max_tokens(10)
            .build()
            .expect("Building rate limiter failed")
    })
}

pub struct HGNCClient {
    attempts: usize,
    api_url: String,
    client: Client,
}

impl HGNCClient {
    pub fn new(attempts: usize, api_url: String) -> Self {
        HGNCClient {
            attempts,
            api_url,
            client: Client::new(),
        }
    }

    fn fetch_request(&self, url: &str, query: &GeneQuery) -> Result<Vec<GeneDoc>, HGNCError> {
        for _ in 0..self.attempts {
            if let Err(duration) = hgnc_rate_limiter().try_wait() {
                sleep(duration);
            }
            let response = self
                .client
                .get(url)
                .header("User-Agent", "PIVOT")
                .header("Accept", "application/json")
                .send();

            if let Ok(response) = response
                && response.status().is_success()
            {
                let gene_response = response.json::<GeneResponse>()?;
                return Ok(gene_response.response.docs);
            }
        }

        Err(HGNCError::HgncAPI {
            gene: query.inner().to_string(),
            attempts: self.attempts,
        })
    }
}

impl HGNCData for HGNCClient {
    fn request_gene_data(&self, query: GeneQuery) -> Result<GeneDoc, HGNCError> {
        let fetch_url = match &query {
            GeneQuery::Symbol(symbol) => format!("{}fetch/symbol/{}", self.api_url, symbol),
            GeneQuery::HgncId(id) => format!("{}fetch/hgnc_id/{}", self.api_url, id),
        };
        let docs = self.fetch_request(&fetch_url, &query)?;

        if docs.len() == 1 {
            Ok(docs.first().unwrap().clone())
        } else {
            Err(HGNCError::UnexpectedNumberOfDocuments {
                identifier: query.inner().to_string(),
                n_found: docs.len(),
                n_expected: 1,
            })
        }
    }
}

impl Default for HGNCClient {
    fn default() -> Self {
        HGNCClient::new(3, "https://rest.genenames.org/".to_string())
    }
}

impl Debug for HGNCClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HGNCClient")
            .field("api_url", &self.api_url)
            .field("rate_limiter", &"<Ratelimiter>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(GeneQuery::Symbol("ZNF3"), "ZNF3", "HGNC:13089")]
    #[case(GeneQuery::HgncId("HGNC:13089"), "ZNF3", "HGNC:13089")]
    fn test_request_gene_data(
        #[case] query: GeneQuery,
        #[case] expected_symbol: String,
        #[case] expected_hgnc_id: String,
    ) {
        let client = HGNCClient::default();

        let gene_doc = client.request_gene_data(query).unwrap();

        assert_eq!(gene_doc.hgnc_id, Some(expected_hgnc_id));
        assert_eq!(gene_doc.symbol, Some(expected_symbol));
    }

    #[rstest]
    #[case(GeneQuery::Symbol("ZNF3"), ("ZNF3", "HGNC:13089"))]
    #[case(GeneQuery::HgncId("HGNC:13089"), ("ZNF3", "HGNC:13089"))]
    fn test_request_gene_identifier_pair(
        #[case] query: GeneQuery,
        #[case] expected_pair: (&str, &str),
    ) {
        let client = HGNCClient::default();

        let gene_doc = client.request_gene_identifier_pair(query).unwrap();

        assert_eq!(gene_doc.0, expected_pair.0);
        assert_eq!(gene_doc.1, expected_pair.1);
    }

    #[rstest]
    fn test_request_hgnc_id() {
        let client = HGNCClient::default();
        let hgnc_id = client.request_hgnc_id(GeneQuery::Symbol("CLOCK")).unwrap();
        assert_eq!(hgnc_id.as_str(), "HGNC:2082");
    }

    #[rstest]
    fn test_request_gene_symbol() {
        let client = HGNCClient::default();
        let gene_symbol = client
            .request_gene_symbol(GeneQuery::HgncId("HGNC:2082"))
            .unwrap();
        assert_eq!(gene_symbol.as_str(), "CLOCK");
    }
}

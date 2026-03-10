use crate::hgvs::{HGVSData, HGVSError, HgvsVariant};
use std::collections::HashMap;

/// A Mock client for the HGVS interface.
///
/// This struct is intended for use in unit testing. Instead of making live HTTP
/// requests to the VariantValidator API, it serves data from an internal `HashMap`.
/// This allows for deterministic testing of components that rely on `HGVSData`.
#[derive(Debug)]
pub struct MockHGVSClient {
    hgvs_variants: HashMap<String, HgvsVariant>,
}

impl MockHGVSClient {
    pub fn new(hgvs_variants: HashMap<String, HgvsVariant>) -> MockHGVSClient {
        MockHGVSClient { hgvs_variants }
    }
}

impl HGVSData for MockHGVSClient {
    fn request_and_validate_hgvs(&self, unvalidated_hgvs: &str) -> Result<HgvsVariant, HGVSError> {
        self.hgvs_variants
            .get(unvalidated_hgvs)
            .cloned()
            .ok_or(HGVSError::MockClient {
                unvalidated_hgvs: unvalidated_hgvs.to_string(),
            })
    }
}

impl Default for MockHGVSClient {
    fn default() -> Self {
        let mut hgvs_variants = HashMap::new();

        //coding autosomal variant
        hgvs_variants.insert(
            "NM_001173464.1:c.2860C>T".to_string(),
            HgvsVariant::new(
                "hg38",
                "chr12",
                39332405,
                "G",
                "A",
                "KIF21A",
                "HGNC:19349",
                "NM_001173464.1",
                "c.2860C>T",
                "NM_001173464.1:c.2860C>T",
                "NC_000012.12:g.39332405G>A",
                Some("NP_001166935.1:p.(Arg954Trp)"),
            ),
        );

        //another coding autosomal variant
        hgvs_variants.insert(
            "NM_001173464.1:c.2861G>A".to_string(),
            HgvsVariant::new(
                "hg38",
                "chr12",
                39332404,
                "C",
                "T",
                "KIF21A",
                "HGNC:19349",
                "NM_001173464.1",
                "c.2861G>A",
                "NM_001173464.1:c.2861G>A",
                "NC_000012.12:g.39332404C>T",
                Some("NP_001166935.1:p.(Arg954Gln)"),
            ),
        );

        //coding x variant
        hgvs_variants.insert(
            "NM_000132.4:c.3637A>T".to_string(),
            HgvsVariant::new(
                "hg38",
                "chrX",
                154930153,
                "T",
                "A",
                "F8",
                "HGNC:3546",
                "NM_000132.4",
                "c.3637A>T",
                "NM_000132.4:c.3637A>T",
                "NC_000023.11:g.154930153T>A",
                Some("NP_000123.1:p.(Ile1213Phe)"),
            ),
        );

        //non-coding variant
        hgvs_variants.insert(
            "NR_002196.1:n.601G>T".to_string(),
            HgvsVariant::new(
                "hg38",
                "chr11",
                1997235,
                "C",
                "A",
                "H19",
                "HGNC:4713",
                "NR_002196.1",
                "n.601G>T",
                "NR_002196.1:n.601G>T",
                "NC_000011.10:g.1997235C>A",
                None::<&str>,
            ),
        );

        //another non-coding variant
        hgvs_variants.insert(
            "NR_002196.1:n.602C>T".to_string(),
            HgvsVariant::new(
                "hg38",
                "chr11",
                1997234,
                "G",
                "A",
                "H19",
                "HGNC:4713",
                "NR_002196.1",
                "n.601C>T",
                "NR_002196.1:n.602C>T",
                "NC_000011.10:g.1997234G>A",
                None::<&str>,
            ),
        );

        //mitochondrial variant
        hgvs_variants.insert(
            "NC_012920.1:m.616T>C".to_string(),
            HgvsVariant::new(
                "hg38",
                "chrM",
                616,
                "T",
                "C",
                "",
                "",
                "NC_012920.1",
                "m.616T>C",
                "NC_012920.1:m.616T>C",
                "NC_012920.1:m.616T>C",
                None::<&str>,
            ),
        );

        MockHGVSClient::new(hgvs_variants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_validate_hgvs_success() {
        let mock = MockHGVSClient::default();

        let unvalidated_hgvs = "NM_001173464.1:c.2860C>T";

        let hgvs_variant = mock.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(hgvs_variant.transcript_hgvs(), unvalidated_hgvs);
    }

    #[rstest]
    fn test_request_gene_data_not_found() {
        let mock = MockHGVSClient::default();

        let invalid_hgvs = "INVALID_HGVS";
        assert!(mock.request_and_validate_hgvs(invalid_hgvs).is_err());
    }
}

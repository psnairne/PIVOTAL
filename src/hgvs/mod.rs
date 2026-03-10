//! # HGVS
//!
//! This module exposes the public types and traits for requesting variant data from VariantValidator.
//!
//! # [`HgvsVariant`]
//!
//! A struct containing data on the genome assembly, chromosome, position, reference and alt bases of the variant, alongside the symbol and ID of the relevant gene, as well HGVS strings in various format for the variant.
//!
//! # [`HGVSData`]
//!
//! A trait consisting of the following method:
//!
//! - `request_and_validate_hgvs(&self, unvalidated_hgvs: &str) -> Result<HgvsVariant, HGVSError>` — validates that the hgvs is accurate and, if so, returns a HgvsVariant object.
//!
//! # [`HGVSClient`]
//!
//! The basic implementation of the HGVSData trait. Make a request to the VariantValidator API and receive a HgvsVariant object if the &str was a valid hgvs.c or hgvs.n variant string.
//!
//! # [`CachedHGVSClient`]
//!
//! A cached implementation of the HGVSData trait. The HgvsVariant objects will be cached and can thereafter be accessed without an API call.
//!
//! # [`MockHGVSClient`]
//!
//! A mocked implementation of the HGVSData trait for tests and CI.
//!
//! # [`AlleleCount`]
//!
//! An enum with two variants Single and Double. This is used for create a VariantInterpretation from a HgvsVariant object.
//!
//! # [`ChromosomalSex`]
//!
//! An enum with the variants X, XX, XXX, XY, XXY, XYY, Unknown. This is used for create a VariantInterpretation from a HgvsVariant object. Note: the chromosomal sex is relevant when determining whether a mutation on the X or Y chromosome is hemizygous or heterozygous.
//!
//! # [`HGVSError`]
//!
//! An enum for errors returned by the API.
//!
//! # Examples
//!
//! ## HGVSClient
//!
//! ```rust
//! use pivotal::hgvs::{HGVSClient, HGVSData};
//!
//! let client = HGVSClient::default();
//! let hgvs_variant = client.request_and_validate_hgvs("NM_001173464.1:c.2860C>T").unwrap();
//! let expected_chr = "chr12".to_string();
//! assert_eq!(hgvs_variant.chr(),expected_chr);
//! ```
//!
//! ## CachedHGVSClient
//!
//! ```rust
//! use pivotal::hgvs::{CachedHGVSClient, HGVSClient, HGVSData};
//!
//! let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
//! let cache_file_path = temp_dir.path().join("cache.hgvs");
//!
//! let client = CachedHGVSClient::new(cache_file_path, HGVSClient::default()).unwrap();
//! let hgvs_variant = client.request_and_validate_hgvs("NR_002196.1:n.601G>T").unwrap();
//! let expected_gene = "H19".to_string();
//! assert_eq!(hgvs_variant.gene_symbol(),expected_gene);
//!
//! // if we request variant data again, the HGVS API will not be used, as the HgvsVariant object has been cached
//! let hgvs_variant = client.request_and_validate_hgvs("NR_002196.1:n.601G>T").unwrap();
//! ```
//!
//! ## Creating VariantInterpretations from HgvsVariant objects
//!
//! ```rust
//! use pivotal::hgvs::{AlleleCount, ChromosomalSex, HGVSClient, HGVSData};
//!
//! let client = HGVSClient::default();
//! let hgvs_variant = client.request_and_validate_hgvs("NM_001173464.1:c.2860C>T").unwrap();
//! let vi = hgvs_variant.create_variant_interpretation(AlleleCount::Single, &ChromosomalSex::XX);
//!
//! let vi_allelic_state = vi.unwrap().variation_descriptor.unwrap().allelic_state.unwrap().label;
//! assert_eq!("heterozygous", vi_allelic_state);
//! ```

pub use cached_hgvs_client::CachedHGVSClient;
pub use enums::AlleleCount;
pub use enums::ChromosomalSex;
pub use error::HGVSError;
pub use hgvs_client::HGVSClient;
pub use hgvs_variant::HgvsVariant;
pub use mock_hgvs_client::MockHGVSClient;
pub use traits::HGVSData;

mod cached_hgvs_client;
mod enums;
mod error;
mod hgvs_client;
mod hgvs_variant;
mod json_schema;
mod mock_hgvs_client;
mod traits;
mod utils;

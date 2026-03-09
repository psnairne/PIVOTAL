use crate::caching::error::CacherError;
use crate::hgvs::enums::{AlleleCount, ChromosomalSex};
use redb::{CommitError, DatabaseError, StorageError, TableError, TransactionError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HGVSError {
    #[error(
        "Variant Validator did not accept submitted HGVS {hgvs}. Validation warnings: {problems:?}"
    )]
    InvalidHgvs { hgvs: String, problems: Vec<String> },
    #[error("Hgvs string {hgvs} not accepted due to format problem: {problem}.")]
    HgvsFormatNotAccepted { hgvs: String, problem: String },
    #[error(
        "VariantValidator response for {hgvs} had a disallowed flag type {flag}. The allowed flag types are: {allowed_flags:?}"
    )]
    DisallowedFlag {
        hgvs: String,
        flag: String,
        allowed_flags: Vec<String>,
    },
    #[error(
        "VariantValidator response for {hgvs} did not have genome_assembly {desired_assembly}. The following assemblies were found instead: {found_assemblies:?}"
    )]
    GenomeAssemblyNotFound {
        hgvs: String,
        desired_assembly: String,
        found_assemblies: Vec<String>,
    },
    #[error(
        "The provided {id_type} {inputted_gene} does not match with the actual gene {actual_gene} of HGVS variant {hgvs}"
    )]
    MismatchingGeneData {
        id_type: String,
        inputted_gene: String,
        hgvs: String,
        actual_gene: String,
    },
    #[error(
        "VariantValidator response for {hgvs} has element {element} with following problem: {problem}"
    )]
    InvalidVariantValidatorResponseElement {
        hgvs: String,
        element: String,
        problem: String,
    },
    #[error(
        "The following data for a HGVS was contradictory: Chromosomal Sex: {chromosomal_sex:?}, AlleleCount: {allele_count:?}, is_x: {is_x}, is_y: {is_y}"
    )]
    ContradictoryAllelicData {
        chromosomal_sex: ChromosomalSex,
        allele_count: AlleleCount,
        is_x: bool,
        is_y: bool,
    },
    #[error("An allele count of {found} was found. Only allele counts of 1 or 2 are allowed.")]
    InvalidAlleleCount { found: u8 },
    #[error(
        "VariantValidator response for {hgvs} could not be deserialized to schema. Error: {err}."
    )]
    DeserializeVariantValidatorResponseToSchema { hgvs: String, err: String },
    #[error(
        "VariantValidatorAPI returned an error on {attempts} attempts to retrieve data about variant {hgvs}"
    )]
    VariantValidatorAPI { hgvs: String, attempts: usize },
    #[error("VariantValidator response for {hgvs} had an unexpected format: {format_issue}")]
    VariantValidatorResponseUnexpectedFormat { hgvs: String, format_issue: String },
    #[error(transparent)]
    CacheDatabase(#[from] DatabaseError),
    #[error(transparent)]
    CacheTransaction(#[from] TransactionError),
    #[error(transparent)]
    CacheCommit(#[from] CommitError),
    #[error(transparent)]
    CacheTable(#[from] TableError),
    #[error(transparent)]
    CacheStorage(#[from] StorageError),
    #[error(transparent)]
    CacherError(#[from] CacherError),
    #[error("Something went wrong when using Mutex: {0}")]
    MutexError(String),
}

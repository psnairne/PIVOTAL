#![allow(unused)]

use crate::caching::redb_cacher::RedbCacher;
use crate::hgnc::{CachedHGNCClient, HGNCClient, HGNCError};
use crate::hgvs::error::HGVSError;
use crate::hgvs::hgvs_client::HGVSClient;
use crate::hgvs::hgvs_variant::HgvsVariant;
use crate::hgvs::traits::HGVSData;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static HGVS_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn hgvs_cache_mutex() -> &'static Mutex<()> {
    HGVS_CACHE_LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_mutex(mutex: &'_ Mutex<()>) -> Result<MutexGuard<'_, ()>, HGVSError> {
    mutex
        .lock()
        .map_err(|e| HGVSError::MutexError(e.to_string()))
}

#[derive(Debug)]
pub struct CachedHGVSClient {
    cacher: RedbCacher<HgvsVariant>,
    hgvs_client: HGVSClient,
}

impl CachedHGVSClient {
    pub fn new(cache_file_path: PathBuf, hgvs_client: HGVSClient) -> Result<Self, HGVSError> {
        let cacher = RedbCacher::new(cache_file_path);
        {
            let _guard = lock_mutex(hgvs_cache_mutex())?;
            cacher.init_cache()?;
        }
        Ok(CachedHGVSClient {
            cacher,
            hgvs_client,
        })
    }

    pub fn new_with_defaults() -> Result<Self, HGVSError> {
        let cacher = RedbCacher::default();
        let hgvs_client = HGVSClient::default();
        {
            let _guard = lock_mutex(hgvs_cache_mutex())?;
            cacher.init_cache()?;
        }
        Ok(CachedHGVSClient {
            cacher,
            hgvs_client,
        })
    }
}

impl HGVSData for CachedHGVSClient {
    fn request_and_validate_hgvs(&self, unvalidated_hgvs: &str) -> Result<HgvsVariant, HGVSError> {
        {
            let _guard = lock_mutex(hgvs_cache_mutex())?;

            let cache = self.cacher.open_cache()?;
            if let Some(hgvs_variant) = self.cacher.find_cache_entry(unvalidated_hgvs, &cache) {
                return Ok(hgvs_variant);
            }
        }

        let hgvs_variant = self
            .hgvs_client
            .request_and_validate_hgvs(unvalidated_hgvs)?;

        {
            let _guard = lock_mutex(hgvs_cache_mutex())?;

            let cache = self.cacher.open_cache()?;
            self.cacher.cache_object(hgvs_variant.clone(), &cache)?;
        }

        Ok(hgvs_variant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caching::traits::Cacheable;
    use redb::{Database as RedbDatabase, ReadableDatabase};
    use rstest::{fixture, rstest};
    use tempfile::TempDir;

    #[fixture]
    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temporary directory")
    }

    #[rstest]
    fn test_cached_hgvs_client(temp_dir: TempDir) {
        let cache_file_path = temp_dir.path().join("cache.hgvs");
        let cached_client = CachedHGVSClient::new(cache_file_path, HGVSClient::default()).unwrap();

        let unvalidated_hgvs = "NM_001173464.1:c.2860C>T";
        let validated_hgvs = cached_client
            .request_and_validate_hgvs(unvalidated_hgvs)
            .unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);

        let cache = cached_client.cacher.open_cache().unwrap();
        let cached_hgvs = cached_client
            .cacher
            .find_cache_entry(unvalidated_hgvs, &cache)
            .unwrap();
        assert_eq!(cached_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }
}

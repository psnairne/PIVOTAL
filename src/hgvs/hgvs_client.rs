#![allow(unused)]

use crate::hgvs::enums::GenomeAssembly;
use crate::hgvs::error::HGVSError;
use crate::hgvs::hgvs_variant::HgvsVariant;
use crate::hgvs::json_schema::{SingleVariantInfo, VariantValidatorResponse};
use crate::hgvs::traits::HGVSData;
use crate::hgvs::utils::{is_c_hgvs, is_m_hgvs, is_n_hgvs};
use ratelimit::Ratelimiter;
use reqwest::blocking::Client;
use serde_json::Value;
use std::fmt::Debug;
use std::string::ToString;
use std::thread::sleep;
use std::time::Duration;

const ALLOWED_FLAGS: [&str; 2] = ["gene_variant", "mitochondrial"];

pub struct HGVSClient {
    rate_limiter: Ratelimiter,
    attempts: usize,
    api_url: String,
    client: Client,
    genome_assembly: GenomeAssembly,
}

impl Default for HGVSClient {
    fn default() -> Self {
        let rate_limiter = Ratelimiter::builder(2, Duration::from_millis(1180))
            .max_tokens(2)
            .build()
            .expect("Building rate limiter failed");
        let api_url =
            "https://rest.variantvalidator.org/VariantValidator/variantvalidator/".to_string();
        HGVSClient::new(
            rate_limiter,
            3,
            api_url.to_string(),
            Client::new(),
            GenomeAssembly::Hg38,
        )
    }
}

impl Debug for HGVSClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HGVSClient")
            .field("rate_limiter", &"<rate limiter>") // cannot debug-print
            .field("api_url", &self.api_url)
            .field("client", &self.client) // cannot debug-print
            .field("genome_assembly", &self.genome_assembly)
            .finish()
    }
}

impl HGVSClient {
    pub fn new(
        rate_limiter: Ratelimiter,
        attempts: usize,
        api_url: String,
        client: Client,
        genome_assembly: GenomeAssembly,
    ) -> Self {
        HGVSClient {
            rate_limiter,
            attempts,
            api_url,
            client,
            genome_assembly,
        }
    }

    pub fn get_fetch_url(&self, transcript: &str, allele: &str) -> String {
        format!(
            "{}/{}/{}%3A{}/{}?content-type=application%2Fjson",
            self.api_url, self.genome_assembly, transcript, allele, transcript
        )
    }

    fn fetch_request(
        &self,
        fetch_url: String,
        unvalidated_hgvs: &str,
    ) -> Result<VariantValidatorResponse, HGVSError> {
        for _ in 0..self.attempts {
            if let Err(duration) = self.rate_limiter.try_wait() {
                sleep(duration);
            }

            let response = self
                .client
                .get(fetch_url.clone())
                .header("User-Agent", "PIVOT")
                .header("Accept", "application/json")
                .send()
                .map_err(|err| HGVSError::FetchRequest {
                    hgvs: unvalidated_hgvs.to_string(),
                    err: err.to_string(),
                })?;

            if response.status().is_success() {
                return response.json::<VariantValidatorResponse>().map_err(|err| {
                    HGVSError::DeserializeVariantValidatorResponseToSchema {
                        hgvs: unvalidated_hgvs.to_string(),
                        err: err.to_string(),
                    }
                });
            }
        }

        Err(HGVSError::VariantValidatorAPI {
            hgvs: unvalidated_hgvs.to_string(),
            attempts: self.attempts,
        })
    }

    fn get_variant_info_for_valid_hgvs(
        unvalidated_hgvs: &str,
        response: VariantValidatorResponse,
    ) -> Result<SingleVariantInfo, HGVSError> {
        if response.flag == "warning" {
            let validation_warnings = response
                .variant_info
                .get("validation_warning_1")
                .ok_or_else(|| HGVSError::VariantValidatorResponseUnexpectedFormat {
                    hgvs: unvalidated_hgvs.to_string(),
                    format_issue:
                        "The response flag was warning but could not access validation warnings."
                            .to_string(),
                })?
                .validation_warnings
                .clone();
            Err(HGVSError::InvalidHgvs {
                hgvs: unvalidated_hgvs.to_string(),
                problems: validation_warnings,
            })
        } else if !ALLOWED_FLAGS.contains(&response.flag.as_str()) {
            Err(HGVSError::DisallowedFlag {
                hgvs: unvalidated_hgvs.to_string(),
                flag: response.flag.clone(),
                allowed_flags: ALLOWED_FLAGS
                    .to_vec()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            })
        } else if !response.variant_info.len() == 1 {
            Err(HGVSError::VariantValidatorResponseUnexpectedFormat {
                hgvs: unvalidated_hgvs.to_string(),
                format_issue:
                    "VariantValidator response should contain information on exactly one variant."
                        .to_string(),
            })
        } else {
            Ok(response.variant_info.values().next().unwrap().clone())
        }
    }
}

impl HGVSData for HGVSClient {
    fn request_and_validate_hgvs(&self, unvalidated_hgvs: &str) -> Result<HgvsVariant, HGVSError> {
        let (transcript, allele) = Self::get_transcript_and_allele(unvalidated_hgvs)?;
        if !is_c_hgvs(allele) && !is_n_hgvs(allele) && !is_m_hgvs(allele) {
            return Err(HGVSError::HgvsFormatNotAccepted {
                hgvs: unvalidated_hgvs.to_string(),
                problem: "Allele did not begin with c. or n. or m.".to_string(),
            });
        }

        let fetch_url = self.get_fetch_url(transcript, allele);

        let response = self.fetch_request(fetch_url.clone(), unvalidated_hgvs)?;

        let variant_info = Self::get_variant_info_for_valid_hgvs(unvalidated_hgvs, response)?;

        let assemblies = variant_info.primary_assembly_loci;

        let assembly = assemblies
            .get(&self.genome_assembly.to_string())
            .ok_or_else(|| HGVSError::GenomeAssemblyNotFound {
                hgvs: unvalidated_hgvs.to_string(),
                desired_assembly: self.genome_assembly.to_string(),
                found_assemblies: assemblies.keys().cloned().collect::<Vec<String>>(),
            })?
            .clone();

        let position_string = assembly.vcf.pos;
        let position = position_string.parse::<u32>().map_err(|_| {
            HGVSError::InvalidVariantValidatorResponseElement {
                hgvs: unvalidated_hgvs.to_string(),
                element: position_string,
                problem: "position should be parseable to u32".to_string(),
            }
        })?;

        let p_hgvs = if variant_info
            .hgvs_predicted_protein_consequence
            .tlr
            .is_empty()
        {
            None
        } else {
            Some(variant_info.hgvs_predicted_protein_consequence.tlr)
        };

        let validated_hgvs = HgvsVariant::new(
            self.genome_assembly.to_string(),
            assembly.vcf.chr,
            position,
            assembly.vcf.reference,
            assembly.vcf.alt,
            variant_info.gene_symbol,
            variant_info.gene_ids.hgnc_id,
            transcript.to_string(),
            allele.to_string(),
            unvalidated_hgvs.to_string(),
            assembly.hgvs_genomic_description,
            p_hgvs,
        );
        Ok(validated_hgvs)
    }
}

impl HGVSClient {
    fn get_transcript_and_allele(unvalidated_hgvs: &str) -> Result<(&str, &str), HGVSError> {
        let split_hgvs = unvalidated_hgvs.split(':').collect::<Vec<&str>>();
        let colon_count = split_hgvs.len() - 1;
        if colon_count != 1 {
            Err(HGVSError::HgvsFormatNotAccepted {
                hgvs: unvalidated_hgvs.to_string(),
                problem: "There must be exactly one colon in a HGVS string.".to_string(),
            })
        } else {
            let transcript = split_hgvs[0];
            let allele = split_hgvs[1];
            Ok((transcript, allele))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hgvs::error::HGVSError;
    use crate::hgvs::hgvs_client::HGVSClient;
    use crate::hgvs::traits::HGVSData;
    use rstest::{fixture, rstest};

    // this forces tests to run sequentially
    #[rstest]
    fn hgvs_client_tests() {
        let client = HGVSClient::default();
        test_request_and_validate_hgvs_c_autosomal(&client);
        test_request_and_validate_hgvs_c_x(&client);
        test_request_and_validate_hgvs_n(&client);
        test_request_and_validate_hgvs_m(&client);
        test_request_and_validate_hgvs_wrong_reference_base_err(&client);
        test_request_and_validate_hgvs_not_c_or_n_hgvs_err(&client);
    }

    fn test_request_and_validate_hgvs_c_autosomal(client: &HGVSClient) {
        let unvalidated_hgvs = "NM_001173464.1:c.2860C>T";
        let validated_hgvs = client.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }

    fn test_request_and_validate_hgvs_c_x(client: &HGVSClient) {
        let unvalidated_hgvs = "NM_000132.4:c.3637A>T";
        let validated_hgvs = client.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }

    fn test_request_and_validate_hgvs_n(client: &HGVSClient) {
        let unvalidated_hgvs = "NR_002196.1:n.601G>T";
        let validated_hgvs = client.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }

    fn test_request_and_validate_hgvs_m(client: &HGVSClient) {
        let unvalidated_hgvs = "NC_012920.1:m.616T>C";
        let validated_hgvs = client.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }

    fn test_request_and_validate_hgvs_wrong_reference_base_err(client: &HGVSClient) {
        let unvalidated_hgvs = "NM_001173464.1:c.2860G>T";
        let result = client.request_and_validate_hgvs(unvalidated_hgvs);
        assert!(matches!(result, Err(HGVSError::InvalidHgvs { .. })));
    }

    fn test_request_and_validate_hgvs_not_c_or_n_hgvs_err(client: &HGVSClient) {
        let unvalidated_hgvs = "NC_000012.12:g.39332405G>A";
        let result = client.request_and_validate_hgvs(unvalidated_hgvs);
        assert!(matches!(
            result,
            Err(HGVSError::HgvsFormatNotAccepted { .. })
        ));
    }
}

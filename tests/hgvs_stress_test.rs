use pivot::hgvs::{HGVSClient, HGVSData};
use rstest::{fixture, rstest};

fn create_hgvs_variants_from_transcript(
    transcript_name: &str,
    transcript_bases: &str,
) -> Vec<String> {
    let mut hgvs_variants = Vec::with_capacity(transcript_bases.len());
    for (i, base) in transcript_bases.chars().enumerate() {
        let pos = i + 1; // HGVS is 1-based
        let ref_base = base.to_ascii_uppercase();
        let alt_base = 'A';

        let hgvs = format!("{}:c.{}{}>{}", transcript_name, pos, ref_base, alt_base);

        hgvs_variants.push(hgvs);
    }

    hgvs_variants
}

// found here: https://www.ncbi.nlm.nih.gov/CCDS/CcdsBrowse.cgi?REQUEST=NUCID&DATA=1677538156
//283 characters
#[fixture]
fn kif21a_transcript_beginning() -> String {
    let str = "ATGTTGGGCGCCCCGGACGAGAGCTCCGTGCGGGTGGCTGTCAGAATAAGACCACAGCTTGCCAAAGAGA
AGATTGAAGGATGCCATATTTGTACATCTGTCACACCAGGAGAGCCTCAGGTCTTCCTAGGGAAAGATAA
GGCTTTTACTTTTGACTATGTATTTGACATTGACTCCCAGCAAGAGCAGATCTACATTCAATGTATAGAA
AAACTAATTGAAGGTTGCTTTGAAGGATACAATGCTACAGTTTTTGCTTATGGACAAACTGGAGCTGGTA"
        .to_string();
    str.replace('\n', "")
}

#[fixture]
fn kif21a_transcript_name() -> String {
    "NM_001173464.2".to_string()
}

#[rstest]
fn hgvs_stress_test(kif21a_transcript_beginning: String, kif21a_transcript_name: String) {
    let client = HGVSClient::default();
    let unvalidated_hgvs_strings = create_hgvs_variants_from_transcript(
        kif21a_transcript_name.as_str(),
        kif21a_transcript_beginning.as_str(),
    );

    for unvalidated_hgvs in &unvalidated_hgvs_strings {
        let validated_hgvs = client.request_and_validate_hgvs(unvalidated_hgvs).unwrap();
        assert_eq!(validated_hgvs.transcript_hgvs(), unvalidated_hgvs);
    }
}

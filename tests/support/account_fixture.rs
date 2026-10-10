//! Shared authenticated public fixture loading; never used by production.
use super::NativeVerifierConfig;
use czkp::{ProofScope, native::SRS_G1_BYTES};
use data_encoding::HEXLOWER;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
/// One genuine upstream-generated account proof.
pub struct Record {
    /// Canonical public field vector.
    pub public_inputs: Vec<[u8; 32]>,
    /// Opaque upstream proof as lowercase hex.
    pub proof: String,
    /// Authenticated circuit and key identity.
    pub proof_scope: ProofScope,
}

/// Read the public fixture and authenticate the setup bytes.
pub fn fixture() -> (NativeVerifierConfig, Vec<Record>) {
    let source = PathBuf::from(
        std::env::var_os("CVFY_FIXTURE_SOURCE").expect("CI must provide real proof fixtures"),
    );
    let directory = source.join(".proof-test");
    let records: Vec<Record> =
        serde_json::from_slice(&fs::read(directory.join("native-records.json")).unwrap()).unwrap();
    assert_eq!(records.len(), 2);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    let key = fs::read(directory.join("vk.bin")).unwrap();
    let mut g1 = fs::read(directory.join("setup/g1.dat")).unwrap();
    let g2 = fs::read(directory.join("setup/g2.dat")).unwrap();
    for (name, bytes) in [("g1.dat", &g1), ("g2.dat", &g2)] {
        let item = manifest["setup"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == name)
            .unwrap();
        assert_eq!(item["bytes"].as_u64().unwrap() as usize, bytes.len());
        assert_eq!(
            item["sha256"].as_str().unwrap(),
            HEXLOWER.encode(&Sha256::digest(bytes))
        );
    }
    g1.truncate(SRS_G1_BYTES);
    let config = NativeVerifierConfig {
        executable: PathBuf::from(
            std::env::var_os("CVFY_BB").expect("CI must build the native worker"),
        ),
        scope: records[0].proof_scope.clone(),
        timeout: Duration::from_secs(180),
        maximum_parallel: 1,
        max_proof_bytes: 1024 * 1024,
        max_public_inputs: 243,
        threads: 1,
        memory_megabytes: 4096,
        verification_key: key,
        srs_g1: g1,
        srs_g2: g2.try_into().unwrap(),
    };
    (config, records)
}

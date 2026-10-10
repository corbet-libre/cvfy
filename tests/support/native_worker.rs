//! Compiled, deliberately refusing transport fixture; never a proof verifier.
#![forbid(unsafe_code)]
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

fn main() {
    let executable = PathBuf::from(std::env::args_os().next().unwrap());
    let mode = executable.file_stem().unwrap().to_str().unwrap();
    if mode == "early_exit" {
        return;
    }
    if mode == "hang" {
        std::fs::write(executable.with_extension("receipt"), "started").unwrap();
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    let mut input = Vec::new();
    std::io::stdin()
        .take(czkp::native::MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut input)
        .unwrap();
    let request: serde_json::Value = serde_json::from_slice(&input).unwrap();
    match mode {
        "batch" => {
            // Only inert transport metadata is recorded in this fixture.
            let count = request["proofs"].as_array().unwrap().len();
            std::fs::write(executable.with_extension("receipt"), count.to_string()).unwrap();
            std::thread::sleep(Duration::from_secs(60));
        }
        "oversize" => {
            std::io::stdout().write_all(&[b'x'; 8192]).unwrap();
        }
        "partial" => print!("{{"),
        "empty" => (),
        "wrong_scope" => println!(
            "{}",
            serde_json::json!({"verified":true,"proofScope":{"circuitDigest":vec![0;32],"verifyingKeyDigest":vec![0;32]}})
        ),
        "extra" => println!(
            "{}",
            serde_json::json!({"verified":true,"proofScope":request["scope"],"unexpected":true})
        ),
        "environment" => {
            assert_eq!(std::env::var("HARDWARE_CONCURRENCY").unwrap(), "1");
            assert_eq!(std::env::var("CZKP_NATIVE_MEMORY_MIB").unwrap(), "256");
            assert!(std::env::var_os("PATH").is_none());
            assert!(std::env::var_os("HOME").is_none());
            std::fs::write(executable.with_extension("receipt"), "controlled").unwrap();
            println!(
                "{}",
                serde_json::json!({"verified":false,"proofScope":request["scope"]})
            );
        }
        _ => {
            eprintln!("diagnostic-private-marker");
            println!(
                "{}",
                serde_json::json!({"verified":false,"proofScope":request["scope"]})
            );
        }
    }
}

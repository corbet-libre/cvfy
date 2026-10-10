use super::NativeVerifierConfig;
use czkp::{Error, native::Verdict};
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn check_deadline(now: Instant, deadline: Instant) -> Result<(), Error> {
    if now >= deadline {
        Err(Error::CryptoProvider)
    } else {
        Ok(())
    }
}

// Every early return kills/reaps before the pipe threads are joined. This
// executable is trusted and must not fork children that retain its pipe handles.
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn verify(
    config: &NativeVerifierConfig,
    bytes: Vec<u8>,
    deadline: Instant,
) -> Result<(), Error> {
    verify_with_completion_clock(config, bytes, deadline, Instant::now)
}

fn verify_with_completion_clock(
    config: &NativeVerifierConfig,
    bytes: Vec<u8>,
    deadline: Instant,
    completion_clock: impl Fn() -> Instant,
) -> Result<(), Error> {
    check_deadline(Instant::now(), deadline)?;
    let mut command = Command::new(&config.executable);
    command.env_clear();
    // Explicit CI-only opt-in; production forwards no inherited environment.
    // This path is host configuration and can never come from a proof request.
    #[cfg(feature = "instrumented-worker")]
    command.envs(std::env::var_os("LLVM_PROFILE_FILE").map(|path| ("LLVM_PROFILE_FILE", path)));
    let child = command
        .env(
            "CZKP_NATIVE_MEMORY_MIB",
            config.memory_megabytes.to_string(),
        )
        .env("HARDWARE_CONCURRENCY", config.threads.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .or(Err(Error::CryptoProvider))?;
    let mut worker = Worker(child);
    let mut stdin = worker.0.stdin.take().ok_or(Error::CryptoProvider)?;
    let stdout = worker.0.stdout.take().ok_or(Error::CryptoProvider)?;
    let writer = thread::Builder::new()
        .spawn(move || stdin.write_all(&bytes))
        .or(Err(Error::CryptoProvider))?;
    let reader = match thread::Builder::new().spawn(move || {
        let mut output = Vec::new();
        stdout.take(4097).read_to_end(&mut output).map(|_| output)
    }) {
        Ok(reader) => reader,
        Err(_) => {
            drop(worker);
            let _ = writer.join();
            return Err(Error::CryptoProvider);
        }
    };
    let status = loop {
        match worker.0.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(2)),
            _ => break None,
        }
    };
    // Even successful exit is refused when observed at/after the deadline.
    let timely = check_deadline(completion_clock(), deadline);
    drop(worker);
    let wrote = writer.join().ok().is_some_and(|result| result.is_ok());
    let output = reader
        .join()
        .or(Err(Error::CryptoProvider))?
        .or(Err(Error::CryptoProvider))?;
    timely?;
    check_deadline(completion_clock(), deadline)?;
    if !wrote || !status.is_some_and(|value| value.success()) || output.len() > 4096 {
        return Err(Error::CryptoProvider);
    }
    let verdict: Verdict = serde_json::from_slice(&output).or(Err(Error::CryptoProvider))?;
    if !verdict.verified || verdict.proof_scope != config.scope {
        return Err(Error::CryptoProvider);
    }
    check_deadline(completion_clock(), deadline)
}

#[cfg(test)]
#[path = "../tests/native_transport.rs"]
mod tests;

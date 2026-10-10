#!/usr/bin/env python3
"""Require the genuine native-proof regression to reject stale completion time."""
from pathlib import Path
import os
import re
import subprocess

path = Path("src/native/transport.rs")
original = path.read_bytes()
name = "native::transport::tests::a_genuine_proof_observed_after_its_deadline_is_refused"
try:
    source = original.decode()
    admission = "    check_deadline(Instant::now(), deadline)?;"
    if source.count(admission) != 1 or source.count("completion_clock()") != 3:
        raise SystemExit("native deadline control source drift")
    source = source.replace(admission, "    let admitted_at = Instant::now();\n    check_deadline(admitted_at, deadline)?;")
    source = source.replace("completion_clock()", "admitted_at")
    path.write_text(source)
    result = subprocess.run(
        ["cargo", "test", "--locked", "--lib", name, "--", "--exact", "--nocapture"],
        capture_output=True, text=True, timeout=900,
        env={**os.environ, "CARGO_TERM_COLOR": "never", "RUST_BACKTRACE": "0"},
    )
    output = result.stdout + result.stderr
    if (result.returncode != 101 or "test result: FAILED" not in output
            or "late completion must not authorize a valid proof" not in output
            or "tests/native_transport.rs:" not in output
            or not re.search(r"0 passed; 1 failed; 0 ignored;", output)
            or "error[E" in output
            or not re.search(r"test " + re.escape(name) + r" \.\.\. FAILED", output)):
        print(output)
        raise SystemExit("stale completion time did not fail the genuine-proof assertion")
    print("Genuine native-proof assertion rejected stale completion time")
finally:
    path.write_bytes(original)

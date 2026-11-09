use std::io::{self, Cursor};
use std::time::{Duration, Instant};

use sqlite_replication_ltx::{decode_delta, decode_snapshot};

const SNAPSHOT: &[u8] = include_bytes!("../../../../conformance/fixtures/ltx-v2/snapshot-4096.ltx");
const LARGE_DELTA: &[u8] = include_bytes!(
    "../../../../conformance/fixtures/ltx-v2/large-transaction-4096/large-delta-2.ltx"
);
const WARMUP: Duration = Duration::from_secs(1);
const SAMPLE_DURATION: Duration = Duration::from_secs(1);
const SAMPLE_COUNT: usize = 5;

fn main() -> io::Result<()> {
    run_benchmark("reference_snapshot", SNAPSHOT, || {
        decode_snapshot(Cursor::new(SNAPSHOT), &mut io::sink()).map(|_| ())
    })?;
    run_benchmark("reference_large_delta", LARGE_DELTA, || {
        decode_delta(Cursor::new(LARGE_DELTA)).map(|_| ())
    })?;
    Ok(())
}

fn run_benchmark<F>(name: &str, encoded: &[u8], mut decode: F) -> io::Result<()>
where
    F: FnMut() -> io::Result<()>,
{
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < WARMUP {
        decode()?;
    }

    println!(
        "{name}: bytes={}, warmup_seconds=1, sample_seconds=1",
        encoded.len()
    );
    for sample in 1..=SAMPLE_COUNT {
        let start = Instant::now();
        let mut operations = 0_u64;
        while start.elapsed() < SAMPLE_DURATION {
            decode()?;
            operations += 1;
        }
        let elapsed = start.elapsed();
        let seconds = elapsed.as_secs_f64();
        let nanos_per_operation = seconds * 1_000_000_000.0 / operations as f64;
        let megabytes_per_second = encoded.len() as f64 * operations as f64 / seconds / 1_000_000.0;
        println!(
            "sample={sample} operations={operations} ns_per_operation={nanos_per_operation:.0} MB_per_second={megabytes_per_second:.2}"
        );
    }
    Ok(())
}

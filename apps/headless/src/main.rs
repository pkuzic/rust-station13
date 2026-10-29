//! Headless scenario runner for automated tests and benchmarks.

fn main() {
    tracing_subscriber::fmt().init();
    tracing::info!(
        tick_rate = rs13_core::TICK_RATE_HZ,
        reference = rs13_classic::REFERENCE_COMMIT,
        "rs13-headless: no scenarios yet"
    );
}

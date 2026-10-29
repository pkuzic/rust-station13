//! Hub / master server. Optional: game servers must stay reachable by direct
//! IP or domain when the hub is down.

fn main() {
    tracing_subscriber::fmt().init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "rs13-hub: not implemented yet"
    );
}

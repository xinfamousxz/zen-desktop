//! Zen desktop app — thin binary wrapper around the shared shell in lib.rs.
//! (The Android app uses the same lib via the mobile entry point.)

fn main() {
    zen::run();
}

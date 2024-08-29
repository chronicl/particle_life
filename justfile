build-release:
    cargo build --release --no-default-features
trace:
    cargo run --features bevy/trace_tracy

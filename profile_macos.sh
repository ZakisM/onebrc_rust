rm -rf ./instruments/onebrc_rust_CPU-Counters.trace
xctrace record --template "CPU Counters" --output ./instruments/onebrc_rust_CPU-Counters.trace --target-stdin=/dev/ttys001 --target-stdout=/dev/ttys001 --launch -- ./target/release/onebrc_rust

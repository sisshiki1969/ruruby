#!/bin/sh
set -x
cargo run --release --features "perf" -- tests/fibo_perf.rb

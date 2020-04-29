#!/bin/sh
set -x
cargo run --release --features "perf" -- tests/app_mandel_perf.rb > /dev/null

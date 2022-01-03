#!/bin/sh
set -x
# cargo test --all-features
cargo build --release
/usr/bin/time ruby bench/benchmark/app_fibo.rb
/usr/bin/time target/release/ruruby bench/benchmark/app_fibo.rb
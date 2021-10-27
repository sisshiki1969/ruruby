#!/bin/sh
set -x
# cargo test --all-features
cargo build --release
/usr/bin/time ruby tests/app_fibo.rb
/usr/bin/time target/release/ruruby tests/app_fibo.rb
#!/bin/sh

cargo build --release --features trace
RUST_BACKTRACE=1 ../mspec/bin/mspec ../spec/core/array -t target/release/ruruby
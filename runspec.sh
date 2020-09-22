#!/bin/sh
cargo build --release --features verbose
../spec/mspec/bin/mspec -t target/release/ruruby
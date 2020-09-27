#!/bin/sh
cargo build --release
../spec/mspec/bin/mspec -t target/release/ruruby ../spec/core/kernel
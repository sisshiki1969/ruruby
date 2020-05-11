#!/bin/sh
set -x
cargo build --release
time ruby tests/block.rb
time ./target/release/ruruby tests/block.rb

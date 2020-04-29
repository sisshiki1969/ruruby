#!/bin/sh
set -x
cargo build --release
time (ruby tests/fibo.rb)
time (./target/release/ruruby tests/fibo.rb)

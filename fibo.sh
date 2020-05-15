#!/bin/sh
set -x
cargo build --release
/usr/bin/time -lp ruby tests/fibo.rb
/usr/bin/time -lp ./target/release/ruruby tests/fibo.rb

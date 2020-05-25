#!/bin/sh
set -x
cargo build --release
/usr/bin/time ruby tests/fibo.rb
/usr/bin/time ./target/release/ruruby tests/fibo.rb

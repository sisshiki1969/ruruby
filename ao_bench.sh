#!/bin/sh
set -x
cargo build --release
/usr/bin/time -lp ruby tests/ao_bench.rb > ao1.ppm
/usr/bin/time -lp ./target/release/ruruby tests/ao_bench.rb > ao.ppm
convert ao.ppm ao.jpg
convert ao1.ppm ao1.jpg

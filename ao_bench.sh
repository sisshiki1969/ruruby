#!/bin/sh
set -x
cargo build --release
time (ruby tests/ao_bench.rb > ao1.ppm)
time (./target/release/ruruby tests/ao_bench.rb > ao.ppm)
convert ao.ppm ao.jpg
convert ao1.ppm ao1.jpg

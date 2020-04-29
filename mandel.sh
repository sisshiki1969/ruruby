#!/bin/sh
set -x
cargo build --release
time (ruby tests/app_mandelbrot.rb > /dev/null)
time (./target/release/ruruby tests/app_mandelbrot.rb > mandel.ppm)
convert mandel.ppm mandel.jpg

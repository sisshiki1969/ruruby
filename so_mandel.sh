#!/bin/sh
set -x
cargo build --release
time (ruby tests/so_mandelbrot.rb > /dev/null)
time (./target/release/ruruby tests/so_mandelbrot.rb > mandel.ppm)
convert mandel.ppm mandel.jpg

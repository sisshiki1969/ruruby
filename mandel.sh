#!/bin/sh
set -x
cargo build --release
/usr/bin/time ruby tests/app_mandelbrot.rb > /dev/null
/usr/bin/time ./target/release/ruruby tests/app_mandelbrot.rb > mandel.ppm
convert mandel.ppm mandel.jpg

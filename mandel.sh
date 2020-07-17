#!/bin/sh
set -x
cargo build --release
/usr/bin/time ruby tests/app_mandelbrot.rb > /dev/null
/usr/bin/time ./target/release/ruruby tests/app_mandelbrot.rb > mandel1.ppm
convert mandel1.ppm mandel1.jpg
/usr/bin/time ruby tests/so_mandelbrot.rb > /dev/null
/usr/bin/time ./target/release/ruruby tests/so_mandelbrot.rb > mandel2.ppm
convert mandel2.ppm mandel2.jpg
/usr/bin/time ruby tests/app_aobench.rb > /dev/null
/usr/bin/time ./target/release/ruruby tests/app_aobench.rb > ao.ppm
convert ao.ppm ao.jpg

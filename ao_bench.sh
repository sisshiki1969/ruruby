#!/bin/sh
set -x
cargo build --release
/usr/bin/time ruby bench/benchmark/app_aobench.rb > ao1.ppm
/usr/bin/time ./target/release/ruruby bench/benchmark/app_aobench.rb > ao.ppm
convert ao.ppm ao.jpg
convert ao1.ppm ao1.jpg
diff ao.jpg ao1.jpg
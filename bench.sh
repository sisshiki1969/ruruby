#!/bin/sh
cargo build --release
ver='3.0.0'
benchmark-driver bench/benchmark/* --rbenv $ver -e 'target/release/ruruby' --output simple > bench.txt

#!/bin/sh
cargo build --release
ver='3.0.1'
benchmark-driver bench/benchmark/* -rbenv $ver -e 'target/release/ruruby' --output simple
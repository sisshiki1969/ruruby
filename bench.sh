#!/bin/sh
cargo build --release
ver='3.0.0'
benchmark-driver bench/benchmark/* -e 'ruruby-noopt' -e 'ruruby-ltoonly' -e 'ruruby-fullopt' --output simple
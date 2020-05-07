#!/bin/sh
set -x
cargo build --release
time ruby ../optcarrot/bin/optcarrot-bench
time target/release/ruruby ../optcarrot/bin/optcarrot-bench
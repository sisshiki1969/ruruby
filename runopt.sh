#!/bin/sh
set -x
cargo build --release
/usr/bin/time -lp ruby ../optcarrot/bin/optcarrot-bench
/usr/bin/time -lp target/release/ruruby ../optcarrot/bin/optcarrot-bench
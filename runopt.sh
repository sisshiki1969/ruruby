#!/bin/sh
set -x
RUST_BACKTRACE=1 cargo run --release -- ../optcarrot/bin/optcarrot-bench

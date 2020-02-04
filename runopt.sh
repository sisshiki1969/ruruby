#!/bin/sh
set -x
RUST_BACKTRACE=1 cargo run --features verbose -- ../optcarrot/bin/optcarrot-bench

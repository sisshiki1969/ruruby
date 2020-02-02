#!/bin/sh
set -x
cargo run --features verbose -- ../optcarrot/bin/optcarrot-bench

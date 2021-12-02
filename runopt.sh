#!/bin/sh
set -x
cargo test --all-features
cargo build --release
/usr/bin/time ruby ../optcarrot/bin/optcarrot-bench
/usr/bin/time target/release/ruruby ../optcarrot/bin/optcarrot-bench
/usr/bin/time ruby ../optcarrot/bin/optcarrot-bench --opt
/usr/bin/time target/release/ruruby ../optcarrot/bin/optcarrot-bench --opt
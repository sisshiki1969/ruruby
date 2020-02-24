#!/bin/sh
set -x
cargo run --release -- ../optcarrot/bin/optcarrot-bench --load-ppu=../optcarrot/ppu.rb
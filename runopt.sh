#!/bin/sh
set -x
cargo build --release
#ruby ../optcarrot/bin/optcarrot-bench --load-ppu=../optcarrot/ppu.rb -b
ruby ../optcarrot/bin/optcarrot-bench -b --opt
#target/release/ruruby ../optcarrot/bin/optcarrot-bench --load-ppu=../optcarrot/ppu.rb -b
target/release/ruruby ../optcarrot/bin/optcarrot-bench -b --opt
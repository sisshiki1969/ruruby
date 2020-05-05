#!/bin/sh
set -x
cargo build --release
#ruby ../optcarrot/bin/optcarrot-bench --load-ppu=../optcarrot/ppu.rb -b
ruby ../optcarrot/bin/optcarrot-bench --opt #> code1.rb
#target/release/ruruby ../optcarrot/bin/optcarrot-bench --load-ppu=../optcarrot/ppu.rb -b
target/release/ruruby ../optcarrot/bin/optcarrot-bench --opt #> code2.rb
#diff code1.rb code2.rb > diff.txt
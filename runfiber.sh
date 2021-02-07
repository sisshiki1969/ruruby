#!/bin/sh
set -x
# cargo test --all-features
cargo build --release
/usr/bin/time ruby tests/fiber_allocate.rb
/usr/bin/time target/release/ruruby tests/fiber_allocate.rb
/usr/bin/time ruby tests/fiber_switch.rb
/usr/bin/time target/release/ruruby tests/fiber_switch.rb
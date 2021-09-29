#!/bin/sh

<<<<<<< HEAD
cargo build --release --features trace-func
../mspec/bin/mspec ../spec/core/array -t target/release/ruruby
=======
cargo build --release
RUST_BACKTRACE=1 ../mspec/bin/mspec ../spec/core/array -t target/release/ruruby
>>>>>>> master

#!/bin/sh

cargo build --release --features trace-func
../mspec/bin/mspec ../spec/core/array -t target/release/ruruby
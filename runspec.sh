#!/bin/sh
cargo build --release
../mspec/bin/mspec ../spec/core/array/append_spec.rb -t target/release/ruruby
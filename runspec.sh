#!/bin/sh
cargo build --release
../mspec/bin/mspec ../spec/library -t target/release/ruruby
#!/bin/sh
cargo build --release
../mspec/bin/mspec ../spec/core -t target/release/ruruby
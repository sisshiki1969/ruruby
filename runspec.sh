#!/bin/sh
cargo build --release
../mspec/bin/mspec ../spec/language -t target/release/ruruby
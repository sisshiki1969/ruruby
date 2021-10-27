#!/bin/sh

cargo build --release
../mspec/bin/mspec ../spec/core/array -t target/release/ruruby

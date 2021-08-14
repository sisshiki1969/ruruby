#!/bin/sh

cargo build --release
../mspec/bin/mspec ../spec/core/integer -t target/release/ruruby
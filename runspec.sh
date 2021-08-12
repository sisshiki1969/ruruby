#!/bin/sh

cargo build --release
../mspec/bin/mspec ../spec/core/true -t target/release/ruruby
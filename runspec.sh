#!/bin/sh
cargo build --release #--features trace
../mspec/bin/mspec ../spec/core -t target/release/ruruby
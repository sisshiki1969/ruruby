#!/bin/sh
cargo build --release #--features trace
../mspec/bin/mspec ../spec/core/false -t target/release/ruruby
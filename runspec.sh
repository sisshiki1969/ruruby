#!/bin/sh
cargo build --release #--features trace
../mspec/bin/mspec ../spec/language -t target/release/ruruby
#!/bin/sh

cargo build --release #--features trace
../mspec/bin/mspec ../spec/core/integer -t target/release/ruruby
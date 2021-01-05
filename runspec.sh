#!/bin/sh

cargo build --release #--features trace
../mspec/bin/mspec ../spec/core/array -t target/release/ruruby
#!/usr/bin/env fish
cargo llvm-cov clean --workspace; and \
cargo llvm-cov -p pudlib -F unstable --no-report; and \
cargo llvm-cov -p puds -F unstable --no-report; and \
cargo llvm-cov -p pudw -F unstable --no-report; and \
cargo llvm-cov -p pudcli -F unstable --no-report; and \
cargo llvm-cov report --lcov --output-path lcov.info; and \
cargo llvm-cov report --html

#!/bin/bash
# Quick test: render first 50 frames with batch mode to prove concept

echo "Testing GPU batch rendering..."
time ./target/release/ttyvid -i assets/687.cast -o /tmp/test_batch.gif --theme fdwm 2>&1 | grep -E "(GPU|frames|seconds)"

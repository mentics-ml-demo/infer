#!/bin/bash
set -e
LOCAL_DIR="$(dirname "${BASH_SOURCE[0]}")"
BASE_DIR=$(realpath "$LOCAL_DIR/..")
source "$LOCAL_DIR"/setenv.sh

cd "$BASE_DIR"

RUST_BACKTRACE=1 cargo run

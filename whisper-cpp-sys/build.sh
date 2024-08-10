#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"
cd whisper.cpp
echo "$@" >make-args
make "$@" -j "$(nproc)" libwhisper.a libcommon.a libggml.a

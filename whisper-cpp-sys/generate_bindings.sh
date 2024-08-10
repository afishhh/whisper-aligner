#!/usr/bin/env bash

if ! hash bindgen 2>/dev/null; then
	nix shell nixpkgs#rust-bindgen -c "$0"
fi

bindgen \
	--raw-line '#![allow(non_upper_case_globals)]' \
	--raw-line '#![allow(non_camel_case_types)]' \
	--raw-line '#![allow(non_snake_case)]' \
	--raw-line 'extern crate link_cplusplus;' \
	whisper.cpp/include/whisper.h -- -I whisper.cpp/ggml/include >src/lib.rs

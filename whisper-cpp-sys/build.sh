#!/usr/bin/env bash

set -euo pipefail

ask_for_continue=

# LTO saves 312 bytes with a cuda build... this is not enough to justify the UX of custom RUSTFLAGS
# if hash clang 2>/dev/null && hash clang++ 2>/dev/null; then
# 	rustc_llvm=$(rustc -vV | grep 'LLVM version' | cut -d ':' -f 2 | xargs)
# 	clang_version=$(clang --version | grep 'clang version' | cut -d ' ' -f 3)
#
# 	read -ra rustc_llvm_parts < <(cut -d '.' -f 1- --output-delimiter=' ' <<<"$rustc_llvm")
# 	read -ra clang_version_parts < <(cut -d '.' -f 1- --output-delimiter=' ' <<<"$clang_version")
#
# 	matches=
# 	if [[ "${rustc_llvm_parts[0]}" == "${clang_version_parts[0]}" ]]; then
# 		matches="major"
# 		if [[ "${rustc_llvm_parts[1]}" == "${clang_version_parts[1]}" ]]; then
# 			matches="majorminor"
# 		fi
# 	fi
#
# 	if [[ -n $matches ]]; then
# 		if [[ "$matches" == "major" ]]; then
# 			echo
# 			echo "warning: only major LLVM version number in rustc and clang matches"
# 			echo "note: hoping this works anyway"
# 			echo
# 		fi
#
# 		echo "clang is installed and matches rustc version, enabling LTO"
# 		echo 'note: you will need to compile with RUSTFLAGS="-C linker-plugin-lto"'
# 		CC=clang
# 		CXX=clang++
# 		CFLAGS+=" -flto=thin"
# 		CXXFLAGS+=" -flto=thin"
# 		NVCCFLAGS+=" -allow-unsupported-compiler"
# 	else
# 		echo "note: clang is installed but version does not match rustc llvm version"
# 		echo "note: LTO will not be enabled"
# 	fi
# 	ask_for_continue=1
# else
	CC="${CC:-cc}"
	CXX="${CXX:-c++}"
	CFLAGS="${CFLAGS:-}"
	CXXFLAGS="${CXXFLAGS:-}"
	NVCCFLAGS="${NVCCFLAGS:-}"
# fi

compiler_kind=$("$CC" --version | head -n 1 | cut -d ' ' -f 1)
if [[ "$compiler_kind" == "clang" ]]; then
	NVCCFLAGS+=" -ccbin=clang"
	openmp=omp
elif [[ "$compiler_kind" == "gcc" ]]; then
	NVCCFLAGS+=" -ccbin=gcc"
	openmp=gomp
else
	echo "error: unknown compiler family: $compiler_kind" >&2
	exit 1
fi

# if [[ -n "$ask_for_continue" ]]; then
# 	read -r -p "continue? [y/N] "
# 	if [[ "$REPLY" != y && "$REPLY" != Y ]]; then
# 		exit 1
# 	fi
# fi

here="$(realpath "$(dirname "$0")")"
cd "$here"
cd whisper.cpp
cat >build-parameters <<EOF
CC: $CC
CXX: $CXX
CFLAGS: $CFLAGS
CXXFLAGS: $CXXFLAGS
NVCCFLAGS: $NVCCFLAGS
USERFLAGS: $*
OPENMP_LIBRARY: $openmp
EOF

export CC CXX CFLAGS CXXFLAGS NVCCFLAGS

PATH="$here/bin:$PATH" make "$@" -j "$(nproc)" libwhisper.a libcommon.a libggml.a

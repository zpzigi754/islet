#!/bin/bash

set -e

export RUSTFLAGS="-Clto -Cembed-bitcode=yes --emit=llvm-bc $RUSTFLAGS" #okay
export RUSTFLAGS="--cfg=verify $RUSTFLAGS"
export RUSTFLAGS="-Warithmetic-overflow -Coverflow-checks=yes $RUSTFLAGS" #okay
export RUSTFLAGS="-Zpanic_abort_tests $RUSTFLAGS"
export RUSTFLAGS="-Cpanic=abort $RUSTFLAGS"

cargo clean
cargo build --features=verifier-klee --target=aarch64-unknown-none-softfloat

# verify using KLEE
rm -rf kleeout
#klee --libc=klee --silent-klee-assume --output-dir=kleeout --warnings-only-to-file ../../out/aarch64-unknown-none-softfloat/debug/deps/fvp*.bc
klee --libc=klee --disable-verify --silent-klee-assume --output-dir=kleeout --warnings-only-to-file ../../out/aarch64-unknown-none-softfloat/debug/deps/fvp*.bc

# view input value for first path
ktest-tool kleeout/test000001.ktest

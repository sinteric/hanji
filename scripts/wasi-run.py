#!/usr/bin/env python3
"""Minimal WASI runner for `cargo test --target wasm32-wasip1` where the
wasmtime CLI is not installed: `pip install wasmtime`, then set
CARGO_TARGET_WASM32_WASIP1_RUNNER=scripts/wasi-run.py. CI uses the wasmtime CLI."""
import os
import sys

import wasmtime

wasm, args = sys.argv[1], sys.argv[1:]
engine = wasmtime.Engine()
store = wasmtime.Store(engine)
wasi = wasmtime.WasiConfig()
wasi.argv = args
wasi.inherit_stdout()
wasi.inherit_stderr()
wasi.env = [(k, v) for k, v in os.environ.items() if k.startswith(("RUST_", "HANJI_"))]
# the workspace root at its own path, so tests can read files by absolute path
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
wasi.preopen_dir(root, root)
wasi.preopen_dir(".", ".")
store.set_wasi(wasi)
linker = wasmtime.Linker(engine)
linker.define_wasi()
module = wasmtime.Module.from_file(engine, wasm)
instance = linker.instantiate(store, module)
try:
    instance.exports(store)["_start"](store)
except wasmtime.ExitTrap as e:
    sys.exit(e.code)

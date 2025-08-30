### Build and run

Use release build to embed web assets into an all-in-one `spore-box.wasm` file
```
cargo build --target=wasm32-wasip2 -r
```

```
wasmtime serve --addr=0.0.0.0:8081 -Scli ./target/wasm32-wasip2/release/spore-box.wasm
```
# Slang Native Playground Example

This is a simple example of using [slang-native-playground](https://github.com/Devon7925/slang-native-playground). See that repo for more information.

## Running

This app can be run by cloning the repository and then executing `cargo run` from the root folder. 

## Web Build

This app supports building for web. To do so run:

```bat
cargo build --target wasm32-unknown-unknown
wasm-bindgen --out-dir target/generated/ --web target/wasm32-unknown-unknown/debug/slang-native-playground-example.wasm 
```

The web build should then be accessible from `index.html`. To see the current web deploy see the [Github Pages Deploy](https://devon7925.github.io/slang-native-playground-example/).



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

## Using this template

After using this template, you will likely want to rename it. To do so change `slang-native-playground-example` to your new name in the following places:

* `Cargo.toml`
* The web build command
* The web build command in `.github\workflows\pages.yml`
* `index.html` on this line:
    * `import init from "./target/generated/slang-native-playground-example.js";`




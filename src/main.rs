use slang_compiler_type_definitions::CompilationResult;
use slang_native_playground::launch;
use slang_shader_macros::compile_shader;

#[cfg(target_family = "wasm")]
mod wasm_workaround {
    unsafe extern "C" {
        pub(super) fn __wasm_call_ctors();
    }
}

fn main() {
    // https://github.com/rustwasm/wasm-bindgen/issues/4446
    #[cfg(target_family = "wasm")]
    unsafe { wasm_workaround::__wasm_call_ctors()};

    let compilation: CompilationResult = compile_shader!("user.slang", ["shaders"]);
    launch(compilation);
}
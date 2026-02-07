use std::env;
use std::path::PathBuf;

fn main() {
    // Set linker arguments
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-Tlinker_script.ld");

    // Re-run if these files change
    println!("cargo:rerun-if-changed=linker_script.ld");
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=../bindgen_wrappers.h");
    println!("cargo:rerun-if-changed=build.rs");

    // Re-run if environment variables change
    println!("cargo:rerun-if-env-changed=PSOC_GNU_PATH");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");

    // Run bindgen
    generate_bindings();
}

fn generate_bindings() {
    // Get paths from environment or use defaults
    let gnu_path = env::var("PSOC_GNU_PATH").unwrap_or_else(|_| {
        r"C:\Program Files (x86)\Cypress\PSoC Creator\4.4\PSoC Creator\import\gnu\arm\5.4.1"
            .to_string()
    });

    let libclang_path =
        env::var("LIBCLANG_PATH").unwrap_or_else(|_| r"C:\Program Files\LLVM\bin".to_string());

    // Set LIBCLANG_PATH for bindgen
    unsafe {
        env::set_var("LIBCLANG_PATH", &libclang_path);
    }

    let gnu_include = format!(r"{}\arm-none-eabi\include", gnu_path);

    println!("cargo:warning=Using PSoC GNU path: {}", gnu_path);
    println!("cargo:warning=Using LIBCLANG path: {}", libclang_path);

    let bindings = bindgen::Builder::default()
        .header("../bindgen_wrappers.h")
        .use_core()
        .ctypes_prefix("cty")
        // Required for modern Rust editions to avoid "extern blocks must be unsafe"
        .generate_inline_functions(true)
        .wrap_unsafe_ops(true)
        // Clang arguments for ARM Cortex-M0
        .clang_args(&[
            "--target=arm",
            "-mfloat-abi=soft",
            "-mcpu=cortex-m0",
            "-mthumb",
            "-I../codegentemp",
            "-I../Generated_Source/PSoC4",
            &format!("-I{}", gnu_include),
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Bindings generated successfully");
}

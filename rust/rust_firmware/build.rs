use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();

    if target.starts_with("thumb") {
        println!("cargo:rustc-link-arg=-Tlink.x");
        println!("cargo:rustc-link-arg=-nostartfiles");
        println!("cargo:rustc-link-arg=-Tlinker_script.ld");
        println!("cargo:rerun-if-changed=linker_script.ld");
        println!("cargo:rerun-if-changed=memory.x");
        generate_bindings();
    }

    println!("cargo:rerun-if-changed=../../hal/bindgen_wrappers.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PSOC_GNU_PATH");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
}

fn libclang_search_path() -> String {
    if let Ok(path) = env::var("LIBCLANG_PATH") {
        return path;
    }

    #[cfg(target_os = "macos")]
    {
        for candidate in [
            "/Library/Developer/CommandLineTools/usr/lib",
            "/opt/homebrew/opt/llvm/lib",
            "/usr/local/opt/llvm/lib",
        ] {
            let dylib = format!("{candidate}/libclang.dylib");
            if std::path::Path::new(&dylib).exists() {
                return candidate.to_string();
            }
        }
        return "/opt/homebrew/opt/llvm/lib".to_string();
    }

    #[cfg(target_os = "windows")]
    {
        return r"C:\Program Files\LLVM\bin".to_string();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "/usr/lib/llvm-14/lib".to_string()
    }
}

fn generate_bindings() {
    let gnu_path = env::var("PSOC_GNU_PATH").unwrap_or_else(|_| {
        r"C:\Program Files (x86)\Cypress\PSoC Creator\4.4\PSoC Creator\import\gnu\arm\5.4.1"
            .to_string()
    });

    let libclang_path = libclang_search_path();

    unsafe {
        env::set_var("LIBCLANG_PATH", &libclang_path);
    }

    let gnu_include = format!(r"{}\arm-none-eabi\include", gnu_path);
    let creator = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../PSoC42rs.cydsn");
    let codegen = creator.join("codegentemp");
    let generated = creator.join("Generated_Source/PSoC4");

    println!("cargo:warning=Using PSoC GNU path: {}", gnu_path);
    println!("cargo:warning=Using LIBCLANG path: {}", libclang_path);

    let bindings = bindgen::Builder::default()
        .header("../../hal/bindgen_wrappers.h")
        .use_core()
        .ctypes_prefix("cty")
        .generate_inline_functions(true)
        .wrap_unsafe_ops(true)
        .clang_args(&[
            "--target=arm",
            "-mfloat-abi=soft",
            "-mcpu=cortex-m0",
            "-mthumb",
            &format!("-I{}", creator.display()),
            &format!("-I{}", codegen.display()),
            &format!("-I{}", generated.display()),
            &format!("-I{}", gnu_include),
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings — install LLVM (libclang)");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:warning=Bindings generated successfully");
}

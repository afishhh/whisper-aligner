fn main() {
    let whisper_dir = std::env::current_dir().unwrap().join("whisper.cpp");
    let make_args = std::fs::read_to_string(whisper_dir.join("make-args")).unwrap_or_default();

    println!("cargo:rustc-link-search={}", whisper_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=whisper");
    println!("cargo:rustc-link-lib=static=common");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib=gomp");

    if make_args.contains("GGML_CUDA") {
        println!("cargo:rustc-link-lib=cuda");
        println!("cargo:rustc-link-lib=cublas");
        println!("cargo:rustc-link-lib=cudart");
        println!("cargo:rustc-link-lib=cublasLt");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

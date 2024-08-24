use std::{collections::BTreeMap};

macro_rules! make_build_parameters_struct {
    // (@rec , $($tt: tt)*) => {
    //     make_build_parameters_struct!(@rec $($tt)*);
    // };
    // (@rec $(,)?) => {
    //
    // };
    ($($name: literal $field: ident: $type: ty),* $(,)?) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        struct BuildParameters {
            $($field: $type),*
        }

        impl BuildParameters {
            fn parse(text: String) -> BuildParameters {
                let mut keyvalues: BTreeMap<_, _> = text.lines().map(|x| {
                    let (key, value) = x.split_at(x.find(':').unwrap());
                    (key.trim().to_string(), value[1..].trim().to_string())
                }).collect();
                #[allow(clippy::extra_unused_type_parameters)]
                const fn one<T>() -> usize { 1 }
                macro_rules! error {
                    () => { panic!("invalid build-parameters file, please rerun build.sh") }
                }
                if keyvalues.len() != $(one::<$type>() +)* 0 {
                    error!()
                }
                BuildParameters {
                    $($field: keyvalues.remove($name).unwrap_or_else(|| error!())),*
                }
            }
        }
    };
}

make_build_parameters_struct! {
    "CC" cc: String,
    "CXX" cxx: String,
    "CFLAGS" cflags: String,
    "CXXFLAGS" cxxflags: String,
    "NVCCFLAGS" nvccflags: String,
    "USERFLAGS" userflags: String,
    "OPENMP_LIBRARY" openmp_library: String,
}

fn main() {
    let whisper_dir = std::env::current_dir().unwrap().join("whisper.cpp");
    let params = BuildParameters::parse(std::fs::read_to_string(whisper_dir.join("build-parameters")).unwrap());

    let all_flags = format!("{} {} {} {}", params.cflags, params.cxxflags, params.nvccflags, params.userflags);

    println!("cargo:rustc-link-search={}", whisper_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=whisper");
    println!("cargo:rustc-link-lib=static=common");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib={}", params.openmp_library);

    if all_flags.contains("GGML_CUDA=") {
        println!("cargo:rustc-link-lib=cuda");
        println!("cargo:rustc-link-lib=cublas");
        println!("cargo:rustc-link-lib=cudart");
        println!("cargo:rustc-link-lib=cublasLt");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

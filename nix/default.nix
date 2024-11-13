naersk:
nix-filter:
whisper-cpp-default:
{ config
, lib
, callPackage

, whisper-cpp-src ? whisper-cpp-default

, rust-bindgen
, pkg-config
, llvmPackages
, ffmpeg

, withCuda ? config.cudaSupport
, cudatoolkit

, withOnnx ? true
, onnxruntime

, cuda_cccl
, cuda_cudart
, addDriverRunpath
, autoAddDriverRunpath
, libcublas
, cuda_nvcc
}:

let
  openmp = llvmPackages.openmp.overrideAttrs (old: {
    patches = [ ];
  });

  bindgenHook = ''
    export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
      $(< ${stdenv.cc}/nix-support/libc-cflags) \
      $(< ${stdenv.cc}/nix-support/cc-cflags) \
      $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
      ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config}"}
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"}
    "
  '';

  # onnxruntime = (pkgs.pkgsStatic.onnxruntime.override { cudaSupport = false; pythonSupport = false; stdenv = pkgs.clangStdenv; }).overrideAttrs (old: { doCheck = false; cmakeFlags = old.cmakeFlags ++ [ "-Donnxruntime_BUILD_SHARED_LIB=OFF" ]; });
  # copyAllStaticLibraries = package: package.overrideAttrs (old: {
  #   postInstall = (old.postInstall or "") + ''
  #     find /build/ -name '*.a' -exec cp {} "$out/lib" \;
  #   '';
  # });
  # onnxruntime = copyAllStaticLibraries ((pkgs.pkgsStatic.onnxruntime.override { cudaSupport = false; pythonSupport = false; stdenv = pkgs.clangStdenv; }).overrideAttrs (old: {
  #   doCheck = false;
  #   cmakeFlags = old.cmakeFlags ++ [ "-Donnxruntime_BUILD_SHARED_LIB=OFF" ];
  # }));
  # inherit (pkgs.pkgsStatic) cpuinfo;
  # pytorch_clog = pkgs.pkgsStatic.stdenv.mkDerivation {
  #   pname = "clog";
  #   inherit (cpuinfo) version src nativeBuildInputs buildInputs checkInputs;
  #   preConfigure = "cd deps/clog";
  #   cmakeFlags = [
  #     (lib.cmakeBool "CLOG_BUILD_TESTS" false)
  #     (lib.cmakeBool "USE_SYSTEM_LIBS" true)
  #   ];
  # };
  # protobuf = copyAllStaticLibraries pkgs.pkgsStatic.protobuf;
  # onnxStaticLibs = [ onnxruntime cpuinfo pytorch_clog protobuf ];
  # extraRustFlags = lib.concatMapStringsSep " " (x: "-L${x}/lib") onnxStaticLibs;

  dynamicLibs = [
    openmp
  ]
  ++ (lib.optional (!ffmpeg.stdenv.hostPlatform.isStatic) ffmpeg)
  ++ (lib.optional withOnnx onnxruntime)
  ++ (lib.optionals withCuda [ cuda_cudart cuda_cccl libcublas ])
  ;

  inherit (llvmPackages) stdenv;
in
(callPackage naersk { inherit stdenv; }).buildPackage {
  # TODO: build whisper.cpp in a separate derivation
  overrideMain = p: p // {
    preBuild = ''
      cd whisper-cpp-sys
      cp -r --no-preserve=all ${whisper-cpp-src} ./whisper.cpp
      NIX_HARDENING_ENABLE= bash build.sh ${if withCuda then "GGML_CUDA=1" else ""}
      cd ..
    '';
    nativeBuildInputs = (p.nativeBuildInputs or [ ])
      ++ (lib.optionals withCuda [ autoAddDriverRunpath cuda_nvcc ]);
    buildInputs = (p.buildInputs or [ ]) ++ dynamicLibs;
  };

  preBuild = bindgenHook;

  src = nix-filter {
    root = ./..;
    include = [
      "whisper-cpp-sys"
      "src"
      "Cargo.toml"
      "Cargo.lock"
    ];
  };

  nativeBuildInputs = [
    pkg-config
    rust-bindgen
    # for llvm-ar
    llvmPackages.libllvm
  ];

  cargoBuildOptions = p: p ++ (lib.optional ffmpeg.stdenv.hostPlatform.isStatic "--features=ffmpeg/static");

  buildInputs = [ ffmpeg ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

  passthru.shellHook = ''
    ${if withOnnx then "export ORT_LIB_LOCATION=${onnxruntime}/lib" else ""}
    export LD_LIBRARY_PATH="${lib.makeLibraryPath ([addDriverRunpath.driverLink] ++ dynamicLibs)}"
  '' + bindgenHook;
}

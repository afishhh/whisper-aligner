{
  description = "A basic flake";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    with flake-utils.lib;
    eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        inherit (pkgs) lib;
        llvmPackages = pkgs.llvmPackages_19;
        openmp = llvmPackages.openmp.overrideAttrs (old: {
          patches = [ ];
        });
        ffmpeg = pkgs.ffmpeg_7;

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

        inherit (llvmPackages) stdenv;
      in
      {
        devShell = pkgs.mkShell.override { inherit stdenv; } {
          nativeBuildInputs = with pkgs; [
            bashInteractive
            rust-bindgen
            pkg-config
            # for llvm-ar
            llvmPackages.libllvm
          ];
          buildInputs = with pkgs; [
            openmp
            # this is funny because it "doesn't belong" in buildInputs
            # but it also has to be in buildInputs
            llvmPackages.libclang
            ffmpeg
            cudatoolkit
          ];

          # preBuild = ''
          #   export RUSTFLAGS="$RUSTFLAGS ${extraRustFlags}"
          # '';

          shellHook = ''
            # export RUSTFLAGS="$RUSTFLAGS ${/*extraRustFlags*/""}"
            export ORT_LIB_LOCATION=${pkgs.onnxruntime}/lib
            export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib"
            export LD_LIBRARY_PATH="${lib.makeLibraryPath [ ffmpeg openmp pkgs.onnxruntime ]}"
            export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
              $(< ${stdenv.cc}/nix-support/libc-cflags) \
              $(< ${stdenv.cc}/nix-support/cc-cflags) \
              $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
              ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
              ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config}"}
              ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"}
            "
          '';
        };
      });
}

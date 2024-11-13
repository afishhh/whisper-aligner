{
  description = "A basic flake";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nix-filter.url = "github:numtide/nix-filter";
  inputs.naersk.url = "github:nix-community/naersk";
  inputs.whisper-cpp = {
    url = "github:ggerganov/whisper.cpp/31aea563a83803c710691fed3e8d700e06ae6788";
    flake = false;
  };

  outputs = { self, nixpkgs, naersk, nix-filter, whisper-cpp, flake-utils }:
    with flake-utils.lib;
    eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        nix-filter' = nix-filter.lib;

        ffmpegStatic = (pkgs.pkgsStatic.ffmpeg.override {
          withHeadlessDeps = false;
          withSmallDeps = false;
          withFullDeps = false;

          withAmf = false;
          withPulse = false;

          withOpus = true;
          withVorbis = true;

          buildAvcodec = true;
          buildAvformat = true;
          buildAvfilter = true;
          buildAvutil = true;
          buildSwresample = true;

          libopus = (pkgs.pkgsStatic.libopus.overrideAttrs {
            # Times out on my system (at least when building multiple packages)
            doCheck = false;
          });
        }).overrideAttrs { doCheck = false; };

        cudaPackages = { inherit (pkgs.cudaPackages) cuda_cccl cuda_cudart libcublas cuda_nvcc; };
        makePackage = args: pkgs.callPackage (import ./nix/default.nix naersk nix-filter' whisper-cpp) (cudaPackages // args);
        cudaArgs = {
          llvmPackages = pkgs.llvmPackages_17;
          withCuda = true;
        };
      in
      {
        packages.default = makePackage { };
        packages.with-cuda = makePackage cudaArgs;
        packages.with-static-ffmpeg = makePackage {
          ffmpeg = ffmpegStatic;
        };
        packages.with-cuda-and-static-ffmpeg = makePackage ({
          ffmpeg = ffmpegStatic;
        } // cudaArgs);
        devShells.default = self.outputs.packages.${system}.default.overrideAttrs (old: {
          shellHook = old.passthru.shellHook;
        });
      });
}

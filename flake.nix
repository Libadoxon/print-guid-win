{
  description = "";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix";
    naersk.url = "github:nix-community/naersk";
  };

  outputs =
    {
      self,
      flake-utils,
      nixpkgs,
      fenix,
      naersk,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        toolchain =
          with fenix.packages.${system};
          combine [
            minimal.rustc
            minimal.cargo
            targets.x86_64-pc-windows-gnu.latest.rust-std
          ];

        naersk' = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      rec {
        devShell = pkgs.mkShell {
          nativeBuildInputs = [ toolchain ];
        };

        defaultPackage = packages.print-guid;

        packages.print-guid = naersk'.buildPackage {
          src = ./.;
          strictDeps = true;

          depsBuildBuild = with pkgs; [
            pkgsCross.mingwW64.stdenv.cc
            pkgsCross.mingwW64.windows.pthreads
          ];

          # Tells Cargo that we're building for Windows.
          # (https://doc.rust-lang.org/cargo/reference/config.html#buildtarget)
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
        };
      }
    );
}

{
  description = "evermemos-rs — Rust rewrite of EverMemOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Use the stable Rust toolchain declared in rust-toolchain.toml if present,
        # otherwise fall back to latest stable.
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        # libclang needed by surrealdb-librocksdb-sys (bindgen)
        llvmPkgs = pkgs.llvmPackages;
      in
      {
        devShells.default = pkgs.mkShell {
          name = "evermemos-rs";

          buildInputs = [
            # Rust
            rustToolchain

            # C/C++ toolchain for RocksDB + bindgen
            llvmPkgs.clang
            llvmPkgs.libclang
            pkgs.llvm

            # Native build deps for RocksDB
            pkgs.cmake
            pkgs.gnumake
            pkgs.pkg-config
            pkgs.openssl
            pkgs.openssl.dev

            # Handy dev tools
            pkgs.cargo-watch
            pkgs.cargo-edit
            pkgs.just

            # C++ standard library (needed at runtime for RocksDB-linked test binaries)
            pkgs.stdenv.cc.cc.lib
          ];

          # bindgen needs to know where libclang.so lives
          LIBCLANG_PATH = "${llvmPkgs.libclang.lib}/lib";

          # Make sure the C compiler used by build scripts is clang
          CC = "${llvmPkgs.clang}/bin/clang";
          CXX = "${llvmPkgs.clang}/bin/clang++";

          # pkg-config can find openssl
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          # libstdc++.so.6 must be visible at runtime (cargo test links against it)
          LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib:${llvmPkgs.libclang.lib}/lib";

          shellHook = ''
            source .venv/bin/activate
            echo "🦀 evermemos-rs dev shell ready"
          '';
        };
      }
    );
}

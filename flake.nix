{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        libPath = pkgs.lib.makeLibraryPath (
          with pkgs; [
            libGL
            libxkbcommon
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ wayland ]
        );

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
  extensions = [ "rust-src" "llvm-tools-preview" ];
};

        nativeBuildInputs = with pkgs; [ rustToolchain ];
        buildInputs = with pkgs; [
          cargo-make
          cargo-deny
          cargo-machete
          cargo-profiler
          samply
          cargo-tauri
          typos
          #cargo-llvm-cov
        ];

        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          shellHook = ''
            export RUST_SRC_PATH=${RUST_SRC_PATH}
            export LD_LIBRARY_PATH=${libPath}:$LD_LIBRARY_PATH

            # Install cargo-llvm-cov if not already present
    if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
      echo "Installing cargo-llvm-cov into local Cargo bin..."
      cargo install cargo-llvm-cov --locked
    fi
          '';
        };
      });
}

# in flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          # 👇 new! note that it refers to the path ./rust-toolchain.toml
          rustToolchain = rust-bin.stable.latest.default;
          # new! 👇
          nativeBuildInputs = with pkgs; [ rustToolchain ];
          # also new! 👇
          buildInputs = with pkgs; [ cargo-make typos markdownlint-cli2 cargo-deny cargo-machete cargo-profiler samply cargo-tauri ];
        in
        with pkgs;
        {
          devShells.default = mkShell {
            # 👇 and now we can just inherit them
            inherit buildInputs nativeBuildInputs;
          };
        }
      );
}

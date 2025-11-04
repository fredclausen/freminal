{
  description = "Freminal dev env — Nix-native pre-commit + xtask";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      git-hooks,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        myRustToolchain = pkgs.rust-bin.fromRustupToolchain {
          channel = "stable";
          components = [
            "rustc"
            "cargo"
            "clippy"
            "rustfmt"
            "rust-analyzer"
            "rust-src"
            "llvm-tools-preview"
          ]; # Add desired components
        };
      in
      {
        checks.pre-commit-check = git-hooks.lib.${system}.run {
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              # keep all files, including dotfiles
              true;
          };

          excludes = [
            "^res/"
            "^./res/"
            "^typos\\.toml$"
            "^speed_tests/.*\\.txt$"
            "^Documents/.*"
          ];

          hooks = {
            # Built-in git-hooks.nix hooks
            check-yaml.enable = true;
            end-of-file-fixer.enable = true;
            trailing-whitespace = {
              enable = true;
              entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/trailing-whitespace-fixer";
            };

            mixed-line-ending = {
              enable = true;
              entry = "${pkgs.python3Packages.pre-commit-hooks}/bin/mixed-line-ending";
              args = [ "--fix=auto" ];
            };

            check-executables-have-shebangs.enable = true;
            check-shebang-scripts-are-executable.enable = true;
            black.enable = true;
            flake8.enable = true;
            nixfmt.enable = true;
            hadolint.enable = true;
            shellcheck.enable = true;
            prettier.enable = true;

            # Hooks that need system packages
            codespell = {
              enable = true;
              entry = "${pkgs.codespell}/bin/codespell";
              args = [ "--ignore-words=.dictionary.txt" ];
              files = "\\.([ch]|cpp|rs|py|sh|txt|md|toml|yaml|yml)$";
            };

            check-github-actions = {
              enable = true;
              entry = "${pkgs.check-jsonschema}/bin/check-jsonschema";
              args = [
                "--builtin-schema"
                "github-actions"
              ];
              files = "^\\.github/actions/.*\\.ya?ml$";
              pass_filenames = true;
            };

            check-github-workflows = {
              enable = true;
              entry = "${pkgs.check-jsonschema}/bin/check-jsonschema";
              args = [
                "--builtin-schema"
                "github-workflows"
              ];
              files = "^\\.github/workflows/.*\\.ya?ml$";
              pass_filenames = true;
            };

            # Rust hooks
            rustfmt = {
              enable = true;
              entry = "${pkgs.rust-bin.stable.latest.default}/bin/cargo";
              args = [
                "fmt"
                "--all"
                "--"
                "--check"
              ];
            };
            clippy = {
              enable = true;
              entry = "${pkgs.rust-bin.stable.latest.default}/bin/cargo";
              args = [
                "clippy"
                "--workspace"
                "--all-targets"
              ];
            };

            xtask-check = {
              enable = true;
              entry = "${pkgs.rust-bin.stable.latest.default}/bin/cargo";
              args = [
                "xtask"
                "ci"
              ];
              pass_filenames = false;
            };
          };
        };

        devShells.default =
          let
            inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
          in
          pkgs.mkShell {
            # Put your Rust toolchain *after* enabledPackages so it wins in PATH
            buildInputs =
              enabledPackages
              ++ [
                myRustToolchain
              ]
              ++ (with pkgs; [
                pre-commit
                check-jsonschema
                codespell
                cargo-deny
                cargo-machete
                cargo-make
                cargo-profiler
                typos
                cachix
                nodePackages.markdownlint-cli2
              ]);

            shellHook = ''
              export PATH="${myRustToolchain}/bin:$PATH"
                # Run git-hooks.nix setup (creates .pre-commit-config.yaml)
                ${shellHook}

                # Your own extras
                alias pre-commit="pre-commit run --all-files"
                alias xtask="cargo run -p xtask --"
            '';
          };

      }
    );
}

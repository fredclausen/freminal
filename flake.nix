{
  description = "Freminal development environment with reproducible pre-commit hooks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    systems.url = "github:nix-systems/default";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      systems,
      git-hooks,
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      checks = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };

          # --- Custom Python Packages ---

          ruamelYaml_0186 = pkgs.python3Packages.buildPythonPackage rec {
            pname = "ruamel.yaml";
            version = "0.18.6";
            pyproject = true;
            build-system = [
              pkgs.python3Packages.setuptools
              pkgs.python3Packages.wheel
            ];
            src = pkgs.fetchPypi {
              pname = "ruamel.yaml";
              inherit version;
              sha256 = "sha256-iyfmohfnhsb75WNNjz8RvGPg+A9qWJDyiGPZxFqsMRs=";
            };
          };

          # replace your preCommitHooksPkg with this
          preCommitHooksPkg = pkgs.python3Packages.buildPythonPackage rec {
            pname = "pre-commit-hooks";
            version = "6.0.0";
            pyproject = true;
            build-system = [
              pkgs.python3Packages.setuptools
              pkgs.python3Packages.wheel
            ];
            src = pkgs.fetchFromGitHub {
              owner = "pre-commit";
              repo = "pre-commit-hooks";
              rev = "v6.0.0";
              sha256 = "sha256-pxtsnRryTguNGYbdiQ55UhuRyJTQvFfaqVOTcCz2jgk=";
            };
            propagatedBuildInputs = [ ruamelYaml_0186 ];
          };

          codespellPkg = pkgs.python3Packages.buildPythonPackage rec {
            pname = "codespell";
            version = "2.4.1";
            pyproject = true;
            build-system = [
              pkgs.python3Packages.setuptools
              pkgs.python3Packages.setuptools-scm
            ];
            src = pkgs.fetchPypi {
              inherit pname version;
              sha256 = "sha256-KZ/NywnSPoHjWmcbvnRtWtfoOFly5l27gzouqsM8AeU=";
            };
          };

          checkJsonschema = pkgs.python3Packages.buildPythonPackage rec {
            pname = "check-jsonschema";
            version = "0.29.4";
            pyproject = true;
            build-system = [
              pkgs.python3Packages.setuptools
              pkgs.python3Packages.wheel
            ];
            src = pkgs.fetchFromGitHub {
              owner = "sirosen";
              repo = pname;
              rev = version;
              sha256 = "sha256-JSz8zwiOnX3nuKypffe0wZ7YoZ7uHH/lAoUgdKfkEO8=";
            };
            propagatedBuildInputs = with pkgs.python3Packages; [
              click
              requests
              jsonschema
              regress
              ruamelYaml_0186
            ];
          };
        in
        {
          pre-commit-check = git-hooks.lib.${system}.run {
            src = ./.;

            # ✅ Global excludes
            excludes = [
              "^res/"
              "^./res/"
              "^typos\\.toml$"
              "^speed_tests/.*\\.txt$"
              "^Documents/.*"
            ];

            hooks = {
              # --- Basic hygiene ---
              check-yaml = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/check-yaml";
              };
              end-of-file-fixer = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/end-of-file-fixer";
              };
              trailing-whitespace = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/trailing-whitespace-fixer";
              };
              requirements-txt-fixer = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/requirements-txt-fixer";
                files = "^(requirements(\\.txt)?|requirements/.*\\.txt)$";
              };
              mixed-line-ending = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/mixed-line-ending";
                args = [ "--fix=auto" ];
              };
              check-executables-have-shebangs = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/check-executables-have-shebangs";
              };
              check-shebang-scripts-are-executable = {
                enable = true;
                entry = "${preCommitHooksPkg}/bin/check-shebang-scripts-are-executable";
              };

              # --- Code quality ---
              hadolint.enable = true;

              prettier = {
                enable = true;
                types_or = [
                  "file"
                  "bash"
                  "sh"
                  "javascript"
                  "jsx"
                  "ts"
                  "tsx"
                ];
                extraPackages = [ pkgs.nodePackages.prettier ];
              };

              codespell = {
                enable = true;
                entry = "${codespellPkg}/bin/codespell";
                args = [ "--ignore-words=.dictionary.txt" ];
                files = "\\.([ch]|cpp|rs|py|sh|txt|md|toml|yaml|yml)$";
              };

              shellcheck.enable = true;

              # --- JSON Schema validation ---
              check-github-actions = {
                enable = true;
                entry = "${checkJsonschema}/bin/check-jsonschema";
                args = [
                  "--builtin-schema"
                  "github-actions"
                ];
                files = "\\.ya?ml$";
              };
              check-github-workflows = {
                enable = true;
                entry = "${checkJsonschema}/bin/check-jsonschema";
                args = [
                  "--builtin-schema"
                  "github-workflows"
                ];
                files = "\\.ya?ml$";
              };

              # --- Rust tooling (explicit toolchain path) ---
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

              # --- Python + Nix formatting ---
              black.enable = true;
              flake8 = {
                enable = true;
                args = [ "--extend-ignore=W503,W504,E501" ];
              };
              nixfmt.enable = true;
            };
          };
        }
      );

      # --- Convenience app to run all hooks ---
      formatter = eachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          inherit (self.checks.${system}.pre-commit-check) config;
          inherit (config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );

      apps = eachSystem (system: {
        pre-commit-run = {
          type = "app";
          program = "${self.formatter.${system}}/bin/pre-commit-run";
        };
      });

      # --- Dev shell ---
      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "llvm-tools-preview"
            ];
          };

          libPath = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              libGL
              libxkbcommon
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ wayland ]
          );

          rustTools = with pkgs; [
            cargo-make
            cargo-deny
            cargo-machete
            cargo-profiler
            samply
            cargo-tauri
            typos
          ];

          inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
        in
        {
          default = pkgs.mkShell {
            buildInputs = [ rustToolchain ] ++ rustTools ++ enabledPackages;
            shellHook = ''
              ${shellHook}
              alias pre-commit="pre-commit-run"
              export RUST_SRC_PATH=${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}
              export LD_LIBRARY_PATH=${libPath}:$LD_LIBRARY_PATH
            '';
          };
        }
      );
    };
}

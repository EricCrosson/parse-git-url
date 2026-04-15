{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    git-hooks,
    ...
  }: let
    systems = [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];

    forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);
  in {
    packages = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      src = builtins.path {
        path = ./.;
        name = "source";
      };
    in {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "parse-git-url";
        version = "0.4.4";
        inherit src;
        cargoHash = "sha256-RrQ3voW2YPLUE3I6RMDb7zCI4LRm85XNN9k8AHSpOUY=";
      };
    });

    checks = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      src = builtins.path {
        path = ./.;
        name = "source";
      };
      pre-commit-check = git-hooks.lib.${system}.run {
        inherit src;
        hooks = {
          actionlint.enable = true;
          alejandra.enable = true;
          prettier.enable = true;
          rustfmt.enable = true;
          taplo.enable = true;
        };
      };
    in {
      default = self.packages.${system}.default;
      inherit pre-commit-check;
    });

    devShells = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      src = builtins.path {
        path = ./.;
        name = "source";
      };
      pre-commit-check = git-hooks.lib.${system}.run {
        inherit src;
        hooks = {
          actionlint.enable = true;
          alejandra.enable = true;
          prettier.enable = true;
          rustfmt.enable = true;
          taplo.enable = true;
        };
      };
    in {
      default = pkgs.mkShell {
        packages =
          (with pkgs; [
            cargo
            clippy
            rust-analyzer
            rustc
            rustfmt
          ])
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

        PROPTEST_CASES = 1000;
        inherit (pre-commit-check) shellHook;
      };
    });
  };
}

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    git-hooks,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
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
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "parse-git-url";
        version = "0.4.4";
        inherit src;
        cargoHash = "sha256-RrQ3voW2YPLUE3I6RMDb7zCI4LRm85XNN9k8AHSpOUY=";
      };

      checks = {
        default = self.packages.${system}.default;
        inherit pre-commit-check;
      };

      devShells.default = pkgs.mkShell {
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
}

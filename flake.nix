{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
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

    forEachSystem = nixpkgs.lib.genAttrs systems;
    perSystem = forEachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      src = builtins.path {
        path = ./.;
        name = "source";
      };
      pre-commit-check = git-hooks.lib.${system}.run {
        inherit src;
        hooks = {
          actionlint.enable = true;
          alejandra = {
            enable = true;
            settings.verbosity = "quiet";
          };
          deadnix.enable = true;
          prettier.enable = true;
          rustfmt.enable = true;
          statix.enable = true;
          taplo.enable = true;
        };
        package = pkgs.prek;
      };
    in {
      inherit pkgs src pre-commit-check;
    });
  in {
    packages = forEachSystem (system: let
      inherit (perSystem.${system}) pkgs src;
    in {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "parse-git-url";
        version = "0.0.0-semantic-release-managed";
        inherit src;
        cargoHash = "sha256-nZfqZO4mgSce8Ebmf/pZvMZ1efyd6sjECEwJSeQjJ7o=";
      };
    });

    checks = forEachSystem (system: let
      inherit (perSystem.${system}) pre-commit-check;
    in {
      inherit (self.packages.${system}) default;
      inherit pre-commit-check;
    });

    devShells = forEachSystem (system: let
      inherit (perSystem.${system}) pkgs pre-commit-check;
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

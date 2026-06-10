{
  description = "The mia-agent";

  nixConfig = {
    extra-substituters = [ "https://mastermach50.cachix.org" ];
    extra-trusted-public-keys = [ "mastermach50.cachix.org-1:tAE8Bm8oMXdo3W+VzuBu2ZahQ03B1Drk4ViZWHcs4j0=" ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { nixpkgs, crane, fenix, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
    in
    {
      packages = nixpkgs.lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          fenixComplete = fenix.packages.${system}.complete;
          craneLib = (crane.mkLib pkgs).overrideToolchain fenixComplete.toolchain;
          mia-agent = pkgs.callPackage ./package.nix { inherit craneLib; };
        in
        {
          default = mia-agent.package;
          mia-agent = mia-agent.package;
        });

      devShells = nixpkgs.lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          fenixComplete = fenix.packages.${system}.complete;
          craneLib = (crane.mkLib pkgs).overrideToolchain fenixComplete.toolchain;
          mia-agent = pkgs.callPackage ./package.nix { inherit craneLib; };
        in
        {
          default = pkgs.mkShell {
            name = "mia-shell";

            MIA_LOG = "mia-agent=TRACE";

            nativeBuildInputs = with pkgs; [
              clang
              mold
              fenixComplete.toolchain
              fenixComplete.rust-analyzer
            ];

            buildInputs = mia-agent.runtimeDeps;
          };
        });
    };
}
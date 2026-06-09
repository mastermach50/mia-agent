{
  description = "The mia-agent";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    { nixpkgs, crane, fenix, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      fenixComplete = fenix.packages.${system}.complete;
      craneLib = (crane.mkLib pkgs).overrideToolchain fenixComplete.toolchain;

      mia-agent = pkgs.callPackage ./package.nix { inherit craneLib; };
    in
    {
      packages.${system}.default = mia-agent.package;
      devShells.${system}.default = pkgs.mkShell {
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
    };
}

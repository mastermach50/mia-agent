{
  description = "Mia Agent, the configurable, easy to use, personal AI agent.";

  nixConfig = {
    extra-substituters = [ "https://mastermach50.cachix.org" ];
    extra-trusted-public-keys = [
      "mastermach50.cachix.org-1:tAE8Bm8oMXdo3W+VzuBu2ZahQ03B1Drk4ViZWHcs4j0="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      crane,
      fenix,
      nixpkgs,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      fenixComplete = fenix.packages."${system}".complete;
      craneLib = (crane.mkLib pkgs).overrideToolchain fenixComplete.toolchain;
    in
    {
      devShells."${system}".mia-dev= pkgs.mkShell {
        name = "mia-shell";

        nativeBuildInputs = with pkgs; [
          fenixComplete.toolchain
          fenixComplete.rust-analyzer
          lldb
        ];
      };

      packages."${system}".default = pkgs.callPackage ./package.nix { inherit craneLib; };
    };
}

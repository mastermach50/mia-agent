{
  description = "The mia-agent";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { nixpkgs, crane, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      craneLib = crane.mkLib pkgs;
    in
    {
      packages.${system}.default = pkgs.callPackage ./package.nix { inherit craneLib; };
      devShells.${system}.default = pkgs.mkShell {
        name = "mia";

        MIA_LOG = "trace";

        nativeBuildInputs = with pkgs; [
          cargo
          rustfmt
          rustc
          mold
          clang
          git
          python313
          rust-analyzer
        ];

        buildInputs = with pkgs; [
          tree
        ];
      };
    };
}

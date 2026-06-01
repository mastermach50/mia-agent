{
  description = "The clara-agent";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        name = "mia";

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
      };
    };
}

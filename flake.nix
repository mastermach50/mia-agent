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
      # Systems we can build FROM (host systems)
      buildSystems = [ "x86_64-linux" "aarch64-linux" ];

      # All cross targets we want to produce
      # Maps: output package attr name -> { rustTarget, crossSystem or null }
      crossTargets = {
        "x86_64-linux"   = { rustTarget = "x86_64-unknown-linux-gnu";    crossSystem = null; };
        "aarch64-linux"  = { rustTarget = "aarch64-unknown-linux-gnu";   crossSystem = "aarch64-multiplatform"; };
        "x86_64-windows" = { rustTarget = "x86_64-pc-windows-gnu";       crossSystem = "x86_64-w64-mingw32"; };
        "aarch64-windows"= { rustTarget = "aarch64-pc-windows-gnullvm";  crossSystem = "aarch64-w64-mingw32"; };
      };

      # Build a package for a given (buildSystem, targetName) pair
      makePackage = buildSystem: targetName:
        let
          target = crossTargets.${targetName};
          isWindows = nixpkgs.lib.hasSuffix "windows" targetName;
          isCross = targetName != buildSystem;

          buildPkgs = import nixpkgs { system = buildSystem; };

          # For cross targets, use pkgsCross; for native, use buildPkgs directly
          crossPkgs =
            if !isCross then buildPkgs
            else if target.crossSystem == null then buildPkgs
            else import nixpkgs {
              system = buildSystem;
              crossSystem = nixpkgs.lib.systems.examples.${target.crossSystem};
            };

          # Build a fenix toolchain that includes the cross Rust target
          fenixPkgs = fenix.packages.${buildSystem};
          toolchain = fenixPkgs.combine [
            fenixPkgs.complete.toolchain
            fenixPkgs.targets.${target.rustTarget}.latest.rust-std
          ];

          craneLib = (crane.mkLib buildPkgs).overrideToolchain toolchain;

          mia-agent = crossPkgs.callPackage ./package.nix {
            inherit craneLib isWindows;
            # On cross builds the CC/linker come from the crossPkgs stdenv
          };
        in
          mia-agent.package;

    in
    {
      packages = nixpkgs.lib.genAttrs buildSystems (buildSystem:
        nixpkgs.lib.mapAttrs
          (targetName: _: makePackage buildSystem targetName)
          crossTargets
        // {
          # Convenience: `default` is the native build
          default = makePackage buildSystem buildSystem;
        }
      );

      devShells = nixpkgs.lib.genAttrs buildSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          fenixPkgs = fenix.packages.${system};
          # Include all cross targets' rust-std in the dev shell
          toolchain = fenixPkgs.combine ([
            fenixPkgs.complete.toolchain
            fenixPkgs.complete.rust-analyzer
          ] ++ (nixpkgs.lib.mapAttrsToList
            (_: t: fenixPkgs.targets.${t.rustTarget}.latest.rust-std)
            crossTargets));
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
          mia-agent = pkgs.callPackage ./package.nix {
            inherit craneLib;
            isWindows = false;
          };
        in
        {
          default = pkgs.mkShell {
            name = "mia-shell";

            MIA_LOG = "mia=DEBUG";

            nativeBuildInputs = with pkgs; [
              clang
              mold
              toolchain
              # Cross-compilation linkers
              pkgsCross.mingwW64.buildPackages.gcc         # x86_64 Windows
              pkgsCross.aarch64-multiplatform.buildPackages.gcc # aarch64 Linux
            ];

            buildInputs = mia-agent.runtimeDeps;
          };
        });
    };
}

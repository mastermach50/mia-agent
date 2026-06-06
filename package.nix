{
  craneLib,
  tree,
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;
    buildInputs = [
      tree
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage ( commonArgs //{ 
  inherit cargoArtifacts;
})

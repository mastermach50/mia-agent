{
  craneLib,
  tree,
  python3,
  clang,
  mold,
  bash
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;
    nativeBuildInputs = [
      clang
      mold
    ];
    buildInputs = [
      tree
      python3
      bash
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage ( commonArgs // { 
  inherit cargoArtifacts;
})

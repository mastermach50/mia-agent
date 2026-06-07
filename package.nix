{
  bash,
  clang,
  craneLib,
  lib,
  makeWrapper,
  mold,
  python3,
  tree,
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;
    nativeBuildInputs = [
      clang
      mold
      makeWrapper
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage ( commonArgs // { 
  inherit cargoArtifacts;

  postInstall = ''
    wrapProgram $out/bin/mia-agent\
      --prefix PATH : ${lib.makeBinPath [ 
        tree
        python3
        bash
      ]}
  '';
})

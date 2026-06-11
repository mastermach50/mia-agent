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

  runtimeDeps = [
      tree
      python3
      bash
    ];

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
{
  package = craneLib.buildPackage ( commonArgs // {
    inherit cargoArtifacts;

    postInstall = ''
      wrapProgram $out/bin/mia\
        --prefix PATH : ${lib.makeBinPath runtimeDeps}
    '';

  });

  # to pull in to the flake devshell
  inherit runtimeDeps;
}

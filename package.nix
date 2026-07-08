{
  bash,
  craneLib,
  fd,
  lib,
  makeWrapper,
  python3,
  pandoc,
  miktex,
  ripgrep,
  tree,
}:
let
  runtimeDeps = [ tree python3 bash ripgrep fd pandoc miktex ];

  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;
    nativeBuildInputs = [ makeWrapper ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
{
  package = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;

    postInstall = ''
      wrapProgram $out/bin/mia \
        --prefix PATH : ${lib.makeBinPath runtimeDeps}
    '';

    meta = {
      description = "The configurable, easy to use, personal AI agent.";
      homepage = "https://github.com/mastermach50/mia-agent";
      licence = lib.licenses.mit;
      maintainers = with lib.maintainers; [ mastermach50 ];
      mainProgram = "mia";
    };
  });

  inherit runtimeDeps;
}

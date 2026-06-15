{
  bash,
  rustPlatform,
  lib,
  makeWrapper,
  fetchFromGitHub,
  python3,
  tree,
  ripgrep,
}:
let
  runtimeDeps = [ tree python3 bash ripgrep ];
in
rustPlatform.buildRustPackage {
  pname = "mia-agent";
  version = "v0.1.0";

  cargoHash = "sha256-ErmMqqlzbgjgmBxWUnt4q4OvAbwjv/qdCkoWW79H9E4=";

  src = fetchFromGitHub {
    owner = "mastermach50";
    repo = "mia-agent";
    rev = "dc3c60df90742fc59c36e6a952cbce76723684be";
    hash = "sha256-QcdV/13VkqBklfef8NYDBn8Ncopm8kF9BBvB3TEa2uM=";
  };

  buildInputs = [
    makeWrapper
  ];

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
}

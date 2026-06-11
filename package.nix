{
  bash,
  clang,
  craneLib,
  isWindows ? false,
  lib,
  makeWrapper,
  mold,
  python3,
  stdenv,
  tree,
}:
let
  # Runtime deps only make sense on Linux (wrapping doesn't apply to .exe)
  runtimeDeps = lib.optionals (!isWindows) [
    tree
    python3
    bash
  ];

  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;

    nativeBuildInputs = lib.optionals (!isWindows) [
      # mold only supports Linux targets
      mold
      makeWrapper
    ] ++ [
      clang
      # On cross builds, crane picks up CC from the stdenv automatically.
      # If targeting Windows, stdenv will be the mingw/llvm one from pkgsCross.
    ];

    # Pass the correct linker to Cargo via environment when cross-compiling
    # to Windows (mingw gcc or llvm-based, depending on the crossSystem used).
    # For Linux targets this is handled by mold/clang from nativeBuildInputs.
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER =
      lib.optionalString (isWindows && stdenv.hostPlatform.isx86_64)
        "${stdenv.cc.targetPrefix}gcc";
    CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER =
      lib.optionalString (isWindows && stdenv.hostPlatform.isAarch64)
        "${stdenv.cc.targetPrefix}clang";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
{
  package = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;

    postInstall = lib.optionalString (!isWindows) ''
      wrapProgram $out/bin/mia \
        --prefix PATH : ${lib.makeBinPath runtimeDeps}
    '';
  });

  # Exposed for the flake devShell
  inherit runtimeDeps;
}

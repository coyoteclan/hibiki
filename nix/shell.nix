{
  mkShell,
  stdenvAdapters,
  cargo-watch,
  hibiki,
}:
mkShell.override (old: {
  stdenv = stdenvAdapters.useMoldLinker old.stdenv;
}) {
  inputsFrom = [hibiki];
  packages = [cargo-watch];
}

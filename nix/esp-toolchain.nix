{ pkgs }:
let
  inherit (pkgs) lib stdenvNoCC fetchurl;
  sources = builtins.fromJSON (builtins.readFile ./esp-sources.json);
  host = sources.hosts.${pkgs.stdenv.hostPlatform.system};
  binaryInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux [
    pkgs.stdenv.cc.cc.lib
    pkgs.zlib
    pkgs.openssl
    # ESP's prebuilt LLVM needs libxml2.so.2 (the 2.13 ABI).
    pkgs.libxml2_13
    pkgs.ncurses
    pkgs.libffi
  ];
  binaryAttrs = {
    nativeBuildInputs = lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.autoPatchelfHook;
    buildInputs = binaryInputs;
    dontConfigure = true;
    dontBuild = true;
    dontStrip = true;
  };
in
{
  rust = stdenvNoCC.mkDerivation (binaryAttrs // {
    pname = "esp-rust";
    version = sources.rustVersion;
    src = fetchurl host.rust;
    rustSrc = fetchurl sources.rustSrc;
    installPhase = ''
      runHook preInstall
      bash install.sh --prefix="$out" --disable-ldconfig --without=rust-docs
      mkdir rust-src-dist
      tar -xf "$rustSrc" -C rust-src-dist --strip-components=1
      bash rust-src-dist/install.sh --prefix="$out" --disable-ldconfig
      runHook postInstall
    '';
  });

  gcc = stdenvNoCC.mkDerivation (binaryAttrs // {
    pname = "xtensa-esp-elf";
    version = sources.gccVersion;
    src = fetchurl host.gcc;
    installPhase = ''
      runHook preInstall
      mkdir -p "$out"
      cp -a . "$out/"
      runHook postInstall
    '';
  });

  clang = stdenvNoCC.mkDerivation (binaryAttrs // {
    pname = "esp-clang-libs";
    version = sources.llvmVersion;
    src = fetchurl host.llvm;
    installPhase = ''
      runHook preInstall
      mkdir -p "$out"
      cp -a . "$out/"
      runHook postInstall
    '';
  });
}

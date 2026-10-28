{
  description = "Development environment for zenoh-nostd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable."1.99.1".minimal.override {
            extensions = [ "clippy" "rustfmt" "rust-src" ];
            targets = [ "wasm32-unknown-unknown" "riscv32imac-unknown-none-elf" "thumbv7em-none-eabihf" ];
          };
          hasEspToolchain = system != "x86_64-darwin";
          esp = import ./nix/esp-toolchain.nix { inherit pkgs; };
          # Rustup resolves named toolchains here without modifying the user's home.
          rustupHome = pkgs.runCommand "zenoh-rustup-home" { } ''
            mkdir -p "$out/toolchains"
            ln -s ${rustToolchain} "$out/toolchains/stable-${pkgs.stdenv.hostPlatform.rust.rustcTarget}"
            ${pkgs.lib.optionalString hasEspToolchain ''
              ln -s ${esp.rust} "$out/toolchains/esp"
            ''}
            cat > "$out/settings.toml" <<EOF
            version = "12"
            default_toolchain = "stable-${pkgs.stdenv.hostPlatform.rust.rustcTarget}"
            profile = "minimal"
            EOF
          '';
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              curl
              espflash
              gcc
              git
              just
              pkg-config
              rustup
            ] ++ pkgs.lib.optionals hasEspToolchain [ esp.gcc ];

            shellHook = ''
              export PATH="${pkgs.rustup}/bin:$PATH"
              export RUSTUP_HOME="${rustupHome}"
              export RUSTUP_TOOLCHAIN="${rustToolchain}"
              ${pkgs.lib.optionalString hasEspToolchain ''
                export LIBCLANG_PATH="${esp.clang}/lib"
              ''}
            '';
          };
        });
    };
}

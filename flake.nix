{
  description = "A reproducible Rust development environment with modern tooling.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.libclang
            pkgs.clang
          ];
          buildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.gtk4
            pkgs.libadwaita
            pkgs.gtk4-layer-shell
            pkgs.libpulseaudio
            pkgs.linux-pam
            pkgs.wlsunset
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linux-pam}/include -I${pkgs.glibc.dev}/include";
          shellHook = ''
            echo "Entering AXIS development environment..."
          '';
        };
      }
    );
}

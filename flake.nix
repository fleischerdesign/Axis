{
  description = "AXIS - A Wayland shell for niri";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = nixpkgs.lib;
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              let
                base = baseNameOf path;
              in
              base == "Cargo.toml"
              || base == "Cargo.lock"
              || (type == "directory" && base == "src")
              || (type == "directory" && base == "crates")
              || (lib.hasPrefix (toString ./src) (toString path))
              || (lib.hasPrefix (toString ./crates) (toString path));
          };
          strictDeps = true;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.clang
            pkgs.meson
            pkgs.ninja
          ];

          buildInputs = [
            pkgs.gtk4
            pkgs.libadwaita
            pkgs.gtk4-layer-shell
            pkgs.libpulseaudio
            pkgs.linux-pam
            pkgs.wl-clipboard
            pkgs.libevdev
          ];

          env = {
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linux-pam}/include -I${pkgs.glibc.dev}/include";
          };
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
      {
        packages.default = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            postInstall = ''
              mkdir -p $out/share/applications
              cat > $out/share/applications/axis-settings.desktop << 'DESKTOP'
              [Desktop Entry]
              Type=Application
              Name=Axis Settings
              Icon=preferences-system-symbolic
              Exec=axis-settings
              Terminal=false
              Categories=Settings;GTK
              Keywords=settings;axis;shell;panel
              Version=1.5
              DESKTOP
            '';
          }
        );

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
            pkgs.cargo
            pkgs.rustc
            pkgs.rust-analyzer
          ];
          buildInputs = commonArgs.buildInputs;
          env = commonArgs.env // {
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
          shellHook = ''
            echo "Entering AXIS development environment..."
          '';
        };
      }
    ))
    // {
      nixosModules.default =
        { pkgs, ... }:
        {
          environment.systemPackages = [ pkgs.wl-clipboard ];
          services.udev.extraRules = ''
            KERNEL=="uinput", GROUP="uinput", MODE="0660", OPTIONS+="static_node=uinput"
            KERNEL=="event*", NAME="input/%k", MODE="0660", GROUP="input"
          '';
          services.avahi = {
            enable = true;
            nssmdns4 = true;
            publish = {
              enable = true;
              addresses = true;
              userServices = true;
            };
          };
          networking.firewall.allowedTCPPorts = [ 7391 ];
        };
    };
}

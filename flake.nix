{
  description = "Keymap manager for wlroots-based compositors";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "wlr-which-key";
          version = "1.2.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            cairo
            pango
            wayland
            libxkbcommon
          ];

          meta = with pkgs.lib; {
            description = "Keymap manager for wlroots-based compositors";
            homepage = "https://github.com/MaxVerevkin/wlr-which-key";
            license = licenses.gpl3Only;
            platforms = platforms.linux;
            mainProgram = "wlr-which-key";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            cairo
            pango
            wayland
            libxkbcommon
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      }
    );
}

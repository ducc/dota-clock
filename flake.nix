{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustc
          cargo
        ];

        buildInputs = with pkgs; [
          gtk4
          gtk4-layer-shell
          glib
          cairo
          pango
          gdk-pixbuf
          graphene
        ];
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "dota-clock";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit nativeBuildInputs buildInputs;

          # Don't wrap with GApps — use system GTK env for fast startup
          dontWrapGApps = true;
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
        };
      }
    );
}

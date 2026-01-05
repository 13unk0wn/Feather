{
  description = "Feather (Rust) flake";

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
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "feather-frontend";
          version = "0.0.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            # filter =
            #   path: type:
            #   let
            #     base = baseNameOf path;
            #   in
            #   base != "target" && base != ".git";
          };

          cargoRoot = "feather_frontend";
          buildAndTestSubdir = "feather_frontend";
          cargoLock.lockFile = ./feather_frontend/Cargo.lock;

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [
            pkgs.openssl
            pkgs.yt-dlp
            pkgs.mpv
          ];
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.yt-dlp
            pkgs.mpv
          ];
        };
      }
    );
}

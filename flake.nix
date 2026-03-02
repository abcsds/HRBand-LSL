{
  description = "HRBand-LSL Rust Development Environment";
  
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            dbus
            cmake
            gcc13
          ];
          shellHook = ''
            export CC=${pkgs.gcc13}/bin/gcc
            export CXX=${pkgs.gcc13}/bin/g++
          '';
        };
      }
    );
}

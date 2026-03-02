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
            liblsl
          ];
          shellHook = ''
            export CC=${pkgs.gcc13}/bin/gcc
            export CXX=${pkgs.gcc13}/bin/g++
            export CXXFLAGS="-DPTHREAD_STACK_MIN=16384"
            export CFLAGS="-DPTHREAD_STACK_MIN=16384"
            export PKG_CONFIG_PATH="${pkgs.liblsl}/lib/pkgconfig:$PKG_CONFIG_PATH"
            export LD_LIBRARY_PATH="${pkgs.liblsl}/lib:$LD_LIBRARY_PATH"
            export RUSTFLAGS="-L ${pkgs.liblsl}/lib -C link-arg=-Wl,-rpath,${pkgs.liblsl}/lib"
            export LSL_INCLUDE_DIR="${pkgs.liblsl}/include"
            export LSL_LIB_DIR="${pkgs.liblsl}/lib"
          '';
        };
      }
    );
}

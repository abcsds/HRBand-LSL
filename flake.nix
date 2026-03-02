{
  description = "HRBand-LSL: Stream BLE heart rate data to Lab Streaming Layer";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Common build inputs needed for the project
        buildInputs = with pkgs; [
          pkg-config
          dbus
          liblsl
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          gcc13
        ];

        # Environment variables for building with liblsl
        buildEnv = {
          CC = "${pkgs.gcc13}/bin/gcc";
          CXX = "${pkgs.gcc13}/bin/g++";
          CXXFLAGS = "-DPTHREAD_STACK_MIN=16384";
          CFLAGS = "-DPTHREAD_STACK_MIN=16384";
          PKG_CONFIG_PATH = "${pkgs.liblsl}/lib/pkgconfig";
          RUSTFLAGS = "-L ${pkgs.liblsl}/lib -C link-arg=-Wl,-rpath,${pkgs.liblsl}/lib";
          LSL_INCLUDE_DIR = "${pkgs.liblsl}/include";
          LSL_LIB_DIR = "${pkgs.liblsl}/lib";
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hrband-lsl";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Set environment variables for the build
          CC = buildEnv.CC;
          CXX = buildEnv.CXX;
          CXXFLAGS = buildEnv.CXXFLAGS;
          CFLAGS = buildEnv.CFLAGS;
          PKG_CONFIG_PATH = buildEnv.PKG_CONFIG_PATH;
          RUSTFLAGS = buildEnv.RUSTFLAGS;
          LSL_INCLUDE_DIR = buildEnv.LSL_INCLUDE_DIR;
          LSL_LIB_DIR = buildEnv.LSL_LIB_DIR;

          meta = with pkgs.lib; {
            description = "Stream BLE heart rate data to Lab Streaming Layer";
            homepage = "https://github.com/example/HRBand-LSL";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
          };
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/hrband-lsl";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = buildInputs ++ (with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            cmake
          ]);

          shellHook = ''
            export CC=${buildEnv.CC}
            export CXX=${buildEnv.CXX}
            export CXXFLAGS="${buildEnv.CXXFLAGS}"
            export CFLAGS="${buildEnv.CFLAGS}"
            export PKG_CONFIG_PATH="${buildEnv.PKG_CONFIG_PATH}:$PKG_CONFIG_PATH"
            export LD_LIBRARY_PATH="${pkgs.liblsl}/lib:$LD_LIBRARY_PATH"
            export RUSTFLAGS="${buildEnv.RUSTFLAGS}"
            export LSL_INCLUDE_DIR="${buildEnv.LSL_INCLUDE_DIR}"
            export LSL_LIB_DIR="${buildEnv.LSL_LIB_DIR}"

            echo "HRBand-LSL development environment"
            echo "  cargo build --release  # Build the project"
            echo "  cargo run              # Run the application"
            echo "  nix build              # Build with nix"
            echo "  nix run                # Run with nix"
          '';
        };
      }
    );
}

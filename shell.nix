{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "sinkd-env";

  buildInputs = with pkgs; [
# rust 
    rustc
    cargo
    rustfmt
    rust-analyzer
    clippy
# everything else
    openssl
    cmake
    mosquitto
    mosquitto.dev  # for headers and pkg-config support
    pkg-config
  ];

  shellHook = ''
    echo "Development environment with Rust, CMake, and Mosquitto is ready!"
  '';
}


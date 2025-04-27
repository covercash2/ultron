{
  description = "Ultron Discord bot and C&C server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Using rust-overlay instead of fenix
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Configure the Rust toolchain using rust-overlay
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-analyzer" "clippy" ];
          targets = [ ];  # Add target triples here if cross-compiling
        };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "ultron-dev";

          buildInputs = with pkgs; [
            bitwarden-cli
            git
            nushell
            openssl
            rustToolchain
          ];

          # Environment variables
          shellHook = ''
            echo "you are now altering Ultron's programming"
            echo "proceed with caution"
          '';

          # For tools that need OpenSSL
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          # You might need these for Rust compilation with certain dependencies
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        # You could also define packages if needed
        packages = {
          # Example: default = self.packages.${system}.ultron;
          # ultron = ...
        };
      }
    );
}

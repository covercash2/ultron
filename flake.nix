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

    # Add crane for easier Rust packaging with Nix
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    let
      # Define the module outside of the per-system scope
      # so it can be accessed directly at self.nixosModules.default
      ultronModule = { config, lib, pkgs, ... }:
      let
        cfg = config.services.ultron;

        # Select the package based on the system
        ultronPackage = self.packages.${pkgs.system}.default;
      in {
        options.services.ultron = {
          enable = lib.mkEnableOption "Ultron Discord bot service";

          user = lib.mkOption {
            type = lib.types.str;
            default = "ultron";
            description = "User account under which Ultron runs";
          };

          group = lib.mkOption {
            type = lib.types.str;
            default = "ultron";
            description = "Group under which Ultron runs";
          };

          environmentFile = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = "Environment file containing Discord tokens and other secrets";
          };
        };

        config = lib.mkIf cfg.enable {
          users.users = lib.mkIf (cfg.user == "ultron") {
            ultron = {
              isSystemUser = true;
              group = cfg.group;
              description = "Ultron Discord bot service user";
              home = "/var/lib/ultron";
              createHome = true;
            };
          };

          users.groups = lib.mkIf (cfg.group == "ultron") {
            ultron = {};
          };

          systemd.services.ultron = {
            description = "Ultron Discord bot";
            wantedBy = [ "multi-user.target" ];
            after = [ "network.target" ];

            serviceConfig = {
              ExecStart = "${ultronPackage}/bin/ultron";
              User = cfg.user;
              Group = cfg.group;
              Restart = "always";
              RestartSec = "10";

              # If an environment file is specified, use it
              EnvironmentFile = lib.mkIf (cfg.environmentFile != null) [ cfg.environmentFile ];

              # Hardening measures
              CapabilityBoundingSet = "";
              DevicePolicy = "closed";
              LockPersonality = true;
              MemoryDenyWriteExecute = true;
              NoNewPrivileges = true;
              PrivateDevices = true;
              PrivateTmp = true;
              ProtectClock = true;
              ProtectControlGroups = true;
              ProtectHome = true;
              ProtectHostname = true;
              ProtectKernelLogs = true;
              ProtectKernelModules = true;
              ProtectKernelTunables = true;
              ProtectSystem = "strict";
              ReadWritePaths = [ "/var/lib/ultron" ];
              RemoveIPC = true;
              RestrictAddressFamilies = [ "AF_INET" "AF_INET6" ];
              RestrictNamespaces = true;
              RestrictRealtime = true;
              RestrictSUIDSGID = true;
              SystemCallArchitectures = "native";
              SystemCallFilter = [ "@system-service" "~@privileged @resources" ];
              UMask = "077";
            };
          };
        };
      };
    in
    {
      # Expose the NixOS module at the top level
      nixosModules.default = ultronModule;
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Configure the Rust toolchain using rust-overlay
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-analyzer" "clippy" ];
        };

        # Set up crane with our rust toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common arguments for the Rust build
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;

          buildInputs = with pkgs; [
            # Add runtime dependencies here
            openssl
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # Environment variables needed during compilation
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
        };

        # Build just the cargo dependencies to improve caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual package
        ultronPackage = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--package ultron"; # Build just the main binary
        });
      in
      {
        # Development shell configuration
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

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        # Package definitions
        packages = {
          ultron = ultronPackage;
          default = ultronPackage;
        };
      }
    );
}

{
  description = "Ultron Discord bot and C&C server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    let
      # Create system-independent module function that takes a package as an argument
      mkUltronModule = ultronPackage: { config, lib, pkgs, ... }:
      let
        cfg = config.services.ultron;
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

          secretsFile = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = "environment file containing Discord tokens and other secrets";
          };

          port = lib.mkOption {
            type = lib.types.int;
            default = 8080;
            description = "port to run the server on";
          };

          rustLog = lib.mkOption {
            type = lib.types.str;
            default = "info";
            description = "the log level of the service. see: https://docs.rs/env_logger/latest/env_logger/#enabling-logging";
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
        };
      };
    in
    {
      # Expose a standalone NixOS module that doesn't depend on packages
      nixosModule = { pkgs, ... }: {
        imports = [
          ({ config, lib, ... }:
            let
              cfg = config.services.ultron;
            in {
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
                    ExecStart = ''
                      ${config.services.ultron.package}/bin/ultron \
                        --port ${config.services.ultron.port} \
                        --rust_log ${config.services.ultron.rustLog} \
                        --secrets ${config.services.ultron.secretsFile}
                    '';
                    User = cfg.user;
                    Group = cfg.group;
                    Restart = "always";
                    RestartSec = "10";

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
            })
        ];
      };
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
          targets = [ ];
        };

        # Set up crane with our rust toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common arguments for the Rust build
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;

          buildInputs = with pkgs; [
            openssl
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

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

          shellHook = ''
            echo "you are now altering Ultron's programming"
            echo "proceed with caution"
          '';

          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          RUST_LOG = "info,ultron=debug";

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        # Package definitions
        packages = {
          ultron = ultronPackage;
          default = ultronPackage;
        };

        # System-specific module that uses the default package
        nixosModules.default = mkUltronModule ultronPackage;
      }
    );
}

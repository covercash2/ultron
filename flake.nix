{
  description = "Ultron Discord bot";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # Fixed crane input
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Properly accessing crane's lib
        craneLib = (crane.mkLib pkgs);

        # Common arguments for crane
        commonArgs = {
          src = craneLib.cleanCargoSource self;

          buildInputs = with pkgs; [
            openssl
            # Add other runtime dependencies as needed
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];
        };

        # Build dependencies separately - allows better caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual package
        ultron = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Additional metadata
          pname = "ultron";
          version = "0.1.0";

          # Augment wrapper path if needed
          postInstall = ''
            wrapProgram $out/bin/ultron \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.openssl ]}
          '';
        });
      in
      {
        # Expose the package
        packages = {
          default = ultron;
          ultron = ultron;
        };

        # Add a check to verify the build works
        checks = {
          inherit ultron;
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          inputsFrom = [ ultron ];
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
            clippy

            just
            just-lsp
            taplo
            typos
            typos-lsp
          ];
        };
      }
    ) // {
      # NixOS module that doesn't depend on system
      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.services.ultron;
        in {
          options.services.ultron = with lib; {
            enable = mkEnableOption "Ultron Discord bot";

            package = mkOption {
              type = types.package;
              description = "The ultron package to use";
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "self.packages.\${pkgs.system}.default";
            };

            user = mkOption {
              type = types.str;
              default = "ultron";
              description = "User account under which Ultron runs";
            };

            group = mkOption {
              type = types.str;
              default = "ultron";
              description = "Group under which Ultron runs";
            };

            secretsFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              example = "/var/lib/ultron/env";
              description = "Environment file containing Discord tokens and other secrets";
            };

            port = mkOption {
              type = types.port;
              default = 8080;
              description = "Port to run the server on";
            };

            rustLog = mkOption {
              type = types.str;
              default = "info";
              example = "info,ultron=debug";
              description = "The log level of the service. See: https://docs.rs/env_logger/latest/env_logger/#enabling-logging";
            };

            dataDir = mkOption {
              type = types.str;
              default = "/var/lib/ultron";
              description = "Directory to store ultron data";
            };
          };

          config = lib.mkIf cfg.enable {
            users.users = lib.mkIf (cfg.user == "ultron") {
              ultron = {
                isSystemUser = true;
                group = cfg.group;
                description = "Ultron Discord bot service user";
                home = cfg.dataDir;
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
                # Pass CLI arguments based on configuration options
                ExecStart = ''
                ${cfg.package}/bin/ultron --port ${toString cfg.port} \
                  --rust-log ${cfg.rustLog} \
                  --secrets ${cfg.secretsFile}
                '';
                User = cfg.user;
                Group = cfg.group;
                Restart = "always";
                RestartSec = "10";

                # Data directory
                StateDirectory = baseNameOf cfg.dataDir;
                StateDirectoryMode = "0750";

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
                ReadWritePaths = [ cfg.dataDir ];
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
    };
}

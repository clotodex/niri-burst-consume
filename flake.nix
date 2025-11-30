{
  description = "Automatically group windows opened in rapid succession in niri";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        {
          config,
          pkgs,
          ...
        }:
        let
          naersk' = pkgs.callPackage inputs.naersk { };
        in
        {
          packages.default = naersk'.buildPackage {
            src = ./.;
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ config.packages.default ];
            packages = with pkgs; [
              cargo
              rustc
              rust-analyzer
              clippy
              rustfmt
            ];
          };
        };

      flake = {
        homeManagerModules.default =
          {
            config,
            lib,
            pkgs,
            ...
          }:
          let
            cfg = config.services.niri-burst-consume;
          in
          {
            options.services.niri-burst-consume = {
              enable = lib.mkEnableOption "niri-burst-consume daemon";

              package = lib.mkOption {
                type = lib.types.package;
                default = inputs.self.packages.${pkgs.system}.default;
                defaultText = lib.literalExpression "inputs.self.packages.\${pkgs.system}.default";
                description = "The niri-burst-consume package to use";
              };

              # TODO: this is not consumed yet
              thresholdMs = lib.mkOption {
                type = lib.types.int;
                default = 500;
                description = "Time window in milliseconds for grouping windows";
              };

              logLevel = lib.mkOption {
                type = lib.types.str;
                default = "error";
                example = "debug";
                description = "Log level (error, info, debug, trace)";
              };
            };

            config = lib.mkIf cfg.enable {
              systemd.user.services.ashell = {
                Unit = {
                  Description = "Niri Burst Consume Daemon";
                  After = [ config.wayland.systemd.target ];
                };

                Service = {
                  ExecStart = "${lib.getExe cfg.package}";
                  Restart = "on-failure";
                  RestartSec = 5;

                  # Resource limits
                  MemoryMax = "50M";
                  MemoryHigh = "30M";
                  CPUQuota = "10%";

                  # Environment
                  Environment = "RUST_LOG=${cfg.logLevel}";
                };

                Install.WantedBy = [ config.wayland.systemd.target ];
              };
            };
          };
      };
    };
}

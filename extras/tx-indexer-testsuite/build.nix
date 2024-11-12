{ inputs, ... }: {
  perSystem = { system, pkgs, config, inputs', self', ... }:
    let
      oura = pkgs.stdenv.mkDerivation {
        src = "${inputs.oura.outPath}";
        name = "oura-v1";
        unpackPhase = ''
          mkdir $out
          cp -r $src/* $out
          cd $out
        '';
      };

      rustFlake =
        inputs.flake-lang.lib.${system}.rustFlake {
          src = ./.;
          crateName = "tx-indexer-testsuite";
          extraSources = [
            config.packages.tx-bakery-rust-src
            config.packages.tx-bakery-ogmios-rust-src
            config.packages.tx-indexer-rust-src
            config.packages.diesel-derive-pg-rust-src
            oura
          ];
          buildInputs = [ pkgs.postgresql_16.lib ];

          devShellTools = [
            self'.packages.tx-indexer-tests
          ];

          devShellHook = config.settings.shell.hook + ''
            ln -sf ${../../tx-indexer}/lib-migrations
          '';
        };
    in
    {
      inherit (rustFlake) devShells packages;
      checks = {
        "tx-indexer-testsuite" = self'.packages.tx-indexer-tests;
      };

      cardano-devnet.initialFunds = {
        "60a5587dc01541d4ad17d7a4416efee274d833f2fc894eef79976a3d06" = 9000000000;
      };

      process-compose.tx-indexer-tests = {
        imports = [
          inputs.services-flake.processComposeModules.default
        ];
        services.postgres."db" = {
          enable = true;
          port = 5555;
          initialDatabases = [
            {
              name = "tx_indexer";
              schemas = [ ];
            }
          ];
        };
        settings.processes = {
          tests = {
            command = "cargo test";
            depends_on = {
              cardano_devnet.condition = "process_healthy";
              ogmios.condition = "process_healthy";
              db.condition = "process_healthy";
              db_migration.condition = "process_completed_successfully";
            };
          };

          cardano_devnet = {
            command = config.packages.cardano-devnet;
            readiness_probe = {
              exec.command = ''${inputs'.cardano-node.packages.cardano-cli}/bin/cardano-cli query tip \
              --socket-path .devnet/node.socket \
              --testnet-magic 42'';
              initial_delay_seconds = 1;
              period_seconds = 1;
            };
          };

          ogmios = {
            command = ''${inputs'.ogmios.packages."ogmios:exe:ogmios"}/bin/ogmios \
            --node-socket .devnet/node.socket \
            --node-config .devnet/config.json
            '';
            readiness_probe = {
              http_get = {
                host = "127.0.0.1";
                port = 1337;
                path = "/health";
              };
              initial_delay_seconds = 2;
              period_seconds = 2;
            };
            depends_on.cardano_devnet.condition = "process_healthy";
          };

          db_migration = {
            command = ''
              ${pkgs.diesel-cli}/bin/diesel migration run \
                --database-url postgres://127.0.0.1:5555/tx_indexer \
                --migration-dir ./lib-migrations

              ${pkgs.diesel-cli}/bin/diesel migration run \
                --database-url postgres://127.0.0.1:5555/tx_indexer \
                --migration-dir ./app-migrations
            '';
            depends_on.db.condition = "process_healthy";
          };

        };
      };

    };

}

{ inputs, ... }:
{
  perSystem =
    { system
    , pkgs
    , config
    , inputs'
    , self'
    , ...
    }:
    let
      rustFlake = inputs.flake-lang.lib.${system}.rustFlake {
        src = ./.;
        crateName = "tx-indexer-testsuite";
        exportTests = true;
        extraSources = [
          config.packages.tx-bakery-rust-src
          config.packages.tx-bakery-ogmios-rust-src
          config.packages.tx-indexer-rust-src
          config.packages.diesel-derive-pg-rust-src
        ];
        buildInputs = [ pkgs.postgresql_16.lib ];

        devShellTools = [
          self'.packages.tx-indexer-tests
        ];

        devShellHook =
          config.settings.shell.hook
          + ''
            ln -sf ${../../tx-indexer}/lib-migrations

            echo "TxIndexer testsuite"
            echo ""
            echo "Run tx-indexer-tests to execute the testsuite."
            echo "or tx-indexer-tests up db_migration ogmios cardano_devnet -t=true to spin up an environment"
            echo ""
          '';
      };
    in
    {
      inherit (rustFlake) devShells packages;

      checks = {
        "tx-indexer-testsuite" = pkgs.stdenv.mkDerivation {
          name = "tx-bakery-testsuite-check";
          phases = [
            "unpackPhase"
            "checkPhase"
            "buildPhase"
          ];
          unpackPhase = ''
            echo "Linking data"
            ln -s ${./wallets} ./wallets
          '';
          checkPhase = ''
            ${self'.packages.tx-bakery-tests}/bin/tx-bakery-tests
          '';
          buildPhase = ''
            mkdir $out
          '';
          doCheck = true;
        };
      };

      cardano-devnet.initialFunds = {
        "60a5587dc01541d4ad17d7a4416efee274d833f2fc894eef79976a3d06" = 9000000000;
      };

      process-compose.tx-indexer-tests = {
        imports = [
          inputs.services-flake.processComposeModules.default
        ];
        cli.environment.PC_DISABLE_TUI = true;
        services.postgres."db" = {
          enable = true;
          port = 5555;
          package = pkgs.postgresql_16;
          dataDir = ".pg";
          initialDatabases = [
            {
              name = "tx_indexer";
              schemas = [ ];
            }
          ];
        };
        settings.processes = {
          tests = {
            command = "find ${self'.packages.tx-indexer-testsuite-rust-test}/bin -maxdepth 1 -type f -executable -exec {} \\;";
            depends_on = {
              cardano_devnet.condition = "process_healthy";
              ogmios.condition = "process_healthy";
              db.condition = "process_healthy";
              db_migration.condition = "process_completed_successfully";
            };
            availability = {
              exit_on_end = true;
              exit_on_skipped = true;
            };
          };

          cardano_devnet = {
            command = config.packages.cardano-devnet;
            readiness_probe = {
              exec.command = ''
                ${inputs'.cardano-node.packages.cardano-cli}/bin/cardano-cli query tip \
                  --socket-path .devnet/node.socket \
                  --testnet-magic 42'';
              initial_delay_seconds = 1;
              period_seconds = 1;
            };
          };

          ogmios = {
            command = ''
              ${inputs'.ogmios.packages."ogmios:exe:ogmios"}/bin/ogmios \
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
                --migration-dir ${../../tx-indexer/lib-migrations}

              ${pkgs.diesel-cli}/bin/diesel migration run \
                --database-url postgres://127.0.0.1:5555/tx_indexer \
                --migration-dir ${./app-migrations}
            '';
            depends_on.db.condition = "process_healthy";
          };

        };
      };

    };

}

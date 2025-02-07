{ inputs, ... }:
{
  perSystem =
    { config
    , system
    , inputs'
    , self'
    , ...
    }:

    let
      rustFlake = inputs.flake-lang.lib."${system}".rustFlake {
        src = ./.;
        crateName = "tx-bakery-tests";
        exportTests = true;

        extraSources = [
          config.packages.tx-bakery-rust-src
          config.packages.tx-bakery-ogmios-rust-src

          # LB base schema and runtime libs
          inputs'.lbf.packages.lbf-prelude-rust
          inputs'.lbf.packages.lbf-plutus-rust

          # Demo API
          config.packages.lbf-tx-bakery-tests-config-api-rust
          config.packages.lbf-tx-bakery-tests-plutus-api-rust
        ];

        data = [
          {
            name = "tx-bakery-test-scripts-config.json";
            path = config.packages.tx-bakery-test-scripts-config;
          }
        ];

        devShellTools = [
          self'.packages.tx-bakery-tests
        ];

        devShellHook =
          config.settings.shell.hook
          + ''
            echo "TxBakery testsuite"
            echo ""
            echo "Run tx-bakery-tests to execute the testsuite."
            echo "or tx-bakery-tests up ogmios cardano_devnet -t=true to spin up an environment"
            echo ""
          '';
      };
    in
    {
      inherit (rustFlake) packages devShells;

      checks = {
        "tx-bakery-testsuite" = self'.packages.tx-bakery-tests;
      };

      cardano-devnet.initialFunds = {
        "60a5587dc01541d4ad17d7a4416efee274d833f2fc894eef79976a3d06" = 9000000000;
      };

      process-compose.tx-bakery-tests = {
        imports = [
          inputs.services-flake.processComposeModules.default
        ];
        cli.environment.PC_DISABLE_TUI = true;
        settings.processes = {
          tests = {
            command = "find ${self'.packages.tx-bakery-tests-rust-test}/bin -maxdepth 1 -type f -executable -exec {} \\;";
            depends_on = {
              cardano_devnet.condition = "process_healthy";
              ogmios.condition = "process_healthy";
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

        };
      };
    };
}

{ inputs, ... }: {
  perSystem = { system, config, inputs', pkgs, ... }:
    let
      commands = import ./commands.nix {
        inherit pkgs;

        schemaDumpIncludePlutus = true;
      };

      rustFlake =
        inputs.flake-lang.lib.${system}.rustFlake {
          src = ./.;
          crateName = "tx-indexer";
          runTests = false;
          extraSources = [
            # LB base schema and runtime libs
            inputs'.lbf.packages.lbf-prelude-rust
            inputs'.lbf.packages.lbf-plutus-rust

            config.packages.tx-bakery-rust-src
            config.packages.tx-bakery-ogmios-rust-src
            config.packages.diesel-derive-pg-rust-src
          ];

          devShellHook = config.settings.shell.hook;

          inherit (commands) devShellTools;

          buildInputs = [ pkgs.postgresql_16 ];

          testTools = with inputs'; [
            ogmios.packages."ogmios:exe:ogmios"
            pkgs.diesel-cli
          ];

          cargoNextestExtraArgs = "-E 'not test(database)'";
        };
    in
    {
      inherit (rustFlake) packages checks devShells;
    };
}

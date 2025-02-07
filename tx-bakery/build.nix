{ inputs, ... }: {
  perSystem = { config, inputs', system, pkgs, ... }:

    let
      rustFlake = inputs.flake-lang.lib."${system}".rustFlake
        {
          src = ./.;
          version = "v2";
          crateName = "tx-bakery";
          cargoNextestExtraArgs = "--no-capture";

          nativeBuildInputs = [ pkgs.openssl pkgs.cacert ];
          buildInputs = [ pkgs.openssl pkgs.cacert ];

          extraSources = [
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

          devShellHook = config.settings.shell.hook;
        };
    in
    {

      inherit (rustFlake) packages checks devShells;

    };
}

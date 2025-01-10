{ inputs, ... }: {
  perSystem = { config, inputs', system, ... }:

    let
      rustFlake = inputs.flake-lang.lib."${system}".rustFlake
        {
          src = ./.;
          version = "v2";
          crateName = "tx-bakery";
          runTests = false;

          extraSources = [
            # LB base schema and runtime libs
            inputs'.lbf.packages.lbf-prelude-rust
            inputs'.lbf.packages.lbf-plutus-rust

            # Demo API
            config.packages.lbf-tx-bakery-tests-config-api-rust
            config.packages.lbf-tx-bakery-tests-plutus-api-rust
          ];

          devShellHook = config.settings.shell.hook;
        };
    in
    {

      inherit (rustFlake) packages checks devShells;

    };
}

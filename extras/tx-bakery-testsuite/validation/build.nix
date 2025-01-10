{ inputs, ... }:
{
  perSystem = { config, system, pkgs, inputs', ... }:
    let
      hsFlake = inputs.flake-lang.lib."${system}".haskellPlutusFlake {
        src = ./.;

        name = "tx-bakery-test-scripts";

        inherit (config.settings.haskell) index-state compiler-nix-name;

        dependencies = [
          # LB base schema and runtimes libs
          # Plutarch
          "${inputs'.lbf.packages.lbf-prelude-plutarch}"
          "${inputs'.lbf.packages.lbf-plutus-plutarch}"
          "${inputs'.lbf.packages.lbr-plutarch-src}"
          # Prelude
          "${inputs'.lbf.packages.lbf-prelude-haskell}"
          "${inputs'.lbf.packages.lbr-prelude-haskell-src}"

          # Plutarch itself
          "${inputs.plutarch}"
          "${inputs.plutarch}/plutarch-ledger-api"

          # Demo API
          "${config.packages.lbf-tx-bakery-tests-plutus-api-plutarch}"
          "${config.packages.lbf-tx-bakery-tests-config-api-haskell}"
        ];

        devShellTools = config.settings.shell.tools;
        devShellHook = config.settings.shell.hook;
      };

    in
    {
      devShells.dev-tx-bakery-test-scripts = hsFlake.devShell;

      packages = {
        # WARN(bladyjoker): We have to pick the hsFlake.packages like this otherwise flake-parts goes into `infinite recursion`.
        tx-bakery-test-scripts-lib = hsFlake.packages."tx-bakery-test-scripts:lib:tx-bakery-test-scripts";

        tx-bakery-test-scripts-cli = hsFlake.packages."tx-bakery-test-scripts:exe:tx-bakery-test-scripts-cli";

        tx-bakery-test-scripts-config = pkgs.stdenv.mkDerivation {
          name = "tx-bakery-test-scripts-config";
          src = ./.;
          buildPhase = ''${config.packages.tx-bakery-test-scripts-cli}/bin/tx-bakery-test-scripts-cli compile --file tx-bakery-tests-config.json'';
          installPhase = "cp tx-bakery-tests-config.json $out";
        };
      };

      inherit (hsFlake) checks;
    };
}

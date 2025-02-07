{ inputs, ... }: {
  perSystem = { config, system, ... }:

    let
      rustFlake = inputs.flake-lang.lib."${system}".rustFlake
        {
          src = ./.;
          version = "v2";
          crateName = "tx-bakery-ogmios";
          runTests = false;

          extraSources = [
            config.packages.tx-bakery-rust-src
          ];

          devShellHook = config.settings.shell.hook;
        };
    in
    {
      inherit (rustFlake) packages checks devShells;
    };
}

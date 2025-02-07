{ inputs, ... }: {
  perSystem = { system, pkgs, config, ... }:
    let
      rustFlake =
        inputs.flake-lang.lib.${system}.rustFlake {
          src = ./.;
          crateName = "diesel-derive-pg";
          runTests = false;

          buildInputs = [ pkgs.postgresql.lib ];

          devShellHook = config.settings.shell.hook;

          generateDocs = false;
        };
    in
    {
      inherit (rustFlake) packages checks devShells;
    };
}

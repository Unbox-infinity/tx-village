{ inputs, ... }: {
  perSystem = { system, config, ... }:
    let
      rustFlake =
        inputs.lbf.lib."${system}".rustFlake {
          src = ./.;
          inherit (inputs) crane;
          crateName = "infinity-query";
          devShellHook = config.settings.shell.hook;
        };
    in
    {
      inherit (rustFlake) packages checks devShells;
    };
}

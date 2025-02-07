{ inputs, ... }: {
  imports = [
    inputs.pre-commit-hooks.flakeModule
  ];
  perSystem = { config, ... }:
    {
      devShells.dev-pre-commit = config.pre-commit.devShell;
      devShells.default = config.pre-commit.devShell;

      pre-commit = {
        settings = {
          excludes = [
          ];

          hooks = {
            nixpkgs-fmt.enable = true;
            deadnix.enable = true;
            statix.enable = true;
            cabal-fmt.enable = true;
            fourmolu.enable = true;
            hlint.enable = true;
            typos.enable = true;
            markdownlint.enable = true;
            dhall-format.enable = true;
            purty.enable = true;
          };

          settings = {
            ormolu.cabalDefaultExtensions = true;
            typos = {
              config = builtins.readFile ./.typos.toml;
              exclude = "fourmolu.yaml";
            };
          };
        };
      };
    };
}

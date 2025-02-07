{
  description = "Cardano Open Oracle Protocol";

  inputs = {
    # LambdaBuffers as source of truth for many inputs
    lbf.url = "github:mlabs-haskell/lambda-buffers";

    # Flake monorepo toolkit
    flake-lang.follows = "lbf/flake-lang";

    # Nix
    nixpkgs.follows = "lbf/nixpkgs";
    flake-parts.follows = "lbf/flake-parts";

    ## Code quality automation
    pre-commit-hooks.follows = "lbf/pre-commit-hooks";
    hci-effects.follows = "lbf/hci-effects";

    # Cardano transaction library (transaction building)
    ctl.follows = "lbf/ctl";

    # Plutarch (Plutus validation scripts)
    plutarch.follows = "lbf/plutarch";

    # Light-weight wrapper around cardano-node
    ogmios.url = "github:mlabs-haskell/ogmios-nix?ref=v6.9.0";

    cardano-nix.url = "github:mlabs-haskell/cardano.nix";

    cardano-devnet-flake.url = "github:szg251/cardano-devnet-flake?ref=szg251/protocol-version-9";

    cardano-node.follows = "cardano-devnet-flake/cardano-node";

    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:juspay/services-flake";
  };
  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./pkgs.nix
        ./settings.nix
        ./pre-commit.nix
        ./hercules-ci.nix
        inputs.process-compose-flake.flakeModule
        inputs.cardano-devnet-flake.flakeModule

        # Libraries
        ./tx-bakery/build.nix
        ./tx-bakery-ogmios/build.nix
        ./extras/diesel-derive-pg/build.nix
        ./extras/tx-bakery-testsuite/api/build.nix
        ./extras/tx-indexer-testsuite/build.nix
        ./tx-indexer/build.nix
        ./ledger-sim/build.nix
        ./dev-net/build.nix

        # Extras
        ./extras/tx-bakery-testsuite/validation/build.nix
        ./extras/tx-bakery-testsuite/tests/build.nix
      ];
      debug = true;
      systems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];
    };
}

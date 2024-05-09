{ inputs, ... }:

{
  # Import all ./zomes/coordinator/*/zome.nix and ./zomes/integrity/*/zome.nix  
  imports = (map (m: "${./.}/zomes/coordinator/${m}/zome.nix")
    (builtins.attrNames (builtins.readDir ./zomes/coordinator)))
    ++ (map (m: "${./.}/zomes/integrity/${m}/zome.nix")
      (builtins.attrNames (builtins.readDir ./zomes/integrity)));
  perSystem = { inputs', config, pkgs, system, lib, self', options, ... }: {
    packages.email_notifications_provider = inputs.hc-infra.outputs.lib.dna {
      holochain = inputs'.holochain;
      dnaManifest = ./workdir/dna.yaml;
      zomes = {
        email_notifications_provider_integrity =
          self'.packages.email_notifications_provider_integrity;
        email_notifications_provider =
          self'.packages.email_notifications_provider_coordinator;
      };
    };
  };
}

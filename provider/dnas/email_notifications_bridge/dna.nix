{ inputs, ... }:

{
  # Import all ./zomes/coordinator/*/zome.nix   
  imports = (map (m: "${./.}/zomes/coordinator/${m}/zome.nix")
    (builtins.attrNames (builtins.readDir ./zomes/coordinator)));

  perSystem = { inputs', self', ... }: {
    packages.email_notifications_bridge-dna = inputs.hc-infra.outputs.lib.dna {
      holochain = inputs'.holochain;
      dnaManifest = ./workdir/dna.yaml;
      zomes = {
        email_notifications_service_integrity =
          self'.packages.email_notifications_service_integrity;
        email_notifications_service =
          self'.packages.email_notifications_service_coordinator;
        email_notifications_bridge = self'.packages.email_notifications_bridge;
      };
    };
  };
}

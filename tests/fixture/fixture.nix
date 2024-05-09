{ inputs, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, self', ... }: {
    packages = rec {
      fixture-dna = inputs.hc-infra.outputs.lib.dna {
        holochain = inputs'.holochain;
        dnaManifest = ./dna.yaml;
        zomes = {
          notifications_integrity =
            inputs'.notifications.packages.notifications_integrity;
          notifications = inputs'.notifications.packages.notifications;
        };
      };
      fixture-happ = inputs.hc-infra.outputs.lib.happ {
        holochain = inputs'.holochain;
        happManifest = ./happ.yaml;
        dnas = {
          fixture_dna = fixture-dna;
          email_notifications_service =
            self'.packages.email_notifications_service-dna;
        };
      };
    };
  };
}

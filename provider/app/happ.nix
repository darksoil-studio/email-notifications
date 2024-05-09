{ inputs, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, self', options, ... }: {
    packages = {
      email_notifications_provider-happ = inputs.hc-infra.outputs.lib.happ {
        holochain = inputs'.holochain;
        happManifest = ./happ.yaml;
        dnas = {
          email_notifications_bridge =
            self'.packages.email_notifications_bridge-dna;
          email_notifications_provider =
            self'.packages.email_notifications_provider;
        };
      };
    };
  };
}

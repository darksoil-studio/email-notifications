{ inputs, rootPath, excludedCrates, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, ... }: {
    packages.email_notifications_service_coordinator =
      inputs.hc-infra.outputs.lib.rustZome {
        inherit excludedCrates;
        workspacePath = rootPath;
        holochain = inputs'.holochain;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

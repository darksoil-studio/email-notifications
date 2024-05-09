{ inputs, rootPath, excludedCrates, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, options, ... }: {
    packages.email_notifications_service_integrity =
      inputs.hc-infra.outputs.lib.rustZome {
        inherit excludedCrates;
        workspacePath = rootPath;
        holochain = inputs'.holochain;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

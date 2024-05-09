{ inputs, rootPath, excludedCrates, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, self', ... }: {
    packages.email_notifications_provider_integrity =
      inputs.hc-infra.outputs.lib.rustZome {
        inherit excludedCrates;
        workspacePath = rootPath;
        holochain = inputs'.holochain;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

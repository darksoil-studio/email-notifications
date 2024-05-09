{ inputs, self, ... }:

{
  flake = {
    lib.make_email_notifications_provider_runner = { network_seed, system }:
      let
        pkgs = import inputs.nixpkgs { inherit system; };
        runner =
          self.outputs.packages.${system}.email_notifications_provider_runner;

      in pkgs.runCommandNoCC "email_notifications_provider_runner" {
        nativeBuildInputs = [ runner pkgs.makeWrapper ];
      } ''
        mkdir $out
        mkdir $out/bin
        # Because we create this ourself, by creating a wrapper
        makeWrapper ${runner}/bin/email_notifications_provider_runner $out/bin/email_notifications_provider_runner \
          --add-flags "--network-seed ${network_seed}"
      '';
  };
  perSystem = { inputs', config, pkgs, system, lib, self', options, ... }: {
    checks.test_email_notifications_provider_runner = let
      test_runner = self.outputs.lib.make_email_notifications_provider_runner {
        inherit system;
        network_seed = "test";
      };
    in pkgs.runCommandNoCC "check_email_notifications_provider_runner" {
      buildInputs = [ test_runner ];
    } ''
      ${test_runner}/bin/email_notifications_provider_runner register-credentials --sender-email-address test@test.org --password my_password --smtp-relay-url somemail.com
      mkdir $out
    '';
    packages.email_notifications_provider_runner = let
      craneLib = inputs.crane.lib.${system};
      crateCargoToml = ./Cargo.toml;
      workspacePath = ../../.;

      cargoToml = builtins.fromTOML (builtins.readFile crateCargoToml);
      crate = cargoToml.package.name;

      commonArgs = {
        strictDeps = true;
        doCheck = false;
        src = craneLib.cleanCargoSource (craneLib.path workspacePath);

        buildInputs = (with pkgs; [
          openssl
          inputs'.holochain.packages.opensslStatic
          sqlcipher
        ]) ++ (lib.optionals pkgs.stdenv.isDarwin
          (with pkgs.darwin.apple_sdk_11_0.frameworks; [
            AppKit
            CoreFoundation
            CoreServices
            Security
            IOKit
          ]));

        nativeBuildInputs = (with pkgs; [
          makeWrapper
          perl
          pkg-config
          inputs'.holochain.packages.goWrapper
        ]) ++ lib.optionals pkgs.stdenv.isDarwin
          (with pkgs; [ xcbuild libiconv ]);

      };

      deps = craneLib.buildDepsOnly (commonArgs // {
        # inherit cargoExtraArgs;
        pname = "email_notifications_provider_runner-workspace";
        version = "workspace";
      });

      bin = craneLib.buildPackage (commonArgs // {
        cargoToml = crateCargoToml;
        cargoLock = workspacePath + /Cargo.lock;
        cargoArtifacts = deps;
        cargoExtraArgs = "-p ${crate} --locked";
        pname = crate;
        version = cargoToml.package.version;
      });
    in pkgs.runCommandNoCC "email_notifications_provider_runner" {
      buildInputs = [ pkgs.makeWrapper bin ];
    } ''
      mkdir $out
      mkdir $out/bin
      # Because we create this ourself, by creating a wrapper
      makeWrapper ${bin}/bin/email_notifications_provider_runner $out/bin/email_notifications_provider_runner \
        --add-flags "${self'.packages.email_notifications_provider-happ}"
    '';

  };

}

{ inputs, ... }:

{
  perSystem = { inputs', config, pkgs, system, lib, self', ... }:
    {

      # checks.email_service_test = pkgs.runCommandNoCC "tryorama-tests" {
      #   # buildInputs = [self'.packages.]
      # } "\n";
    };
}

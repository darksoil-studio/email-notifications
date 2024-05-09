{ ... }:

{
  imports = [ ./app/happ.nix ./runner/runner.nix ]
    ++ (map (m: "${./.}/dnas/${m}/dna.nix")
      (builtins.attrNames (builtins.readDir ./dnas)));
}

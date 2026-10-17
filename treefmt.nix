{
  projectRootFile = "flake.nix";
  programs = {
    deno.enable = true;
    nixfmt.enable = true;
    rustfmt = {
      enable = true;
      edition = "2024";
    };
    taplo.enable = true;
  };
}

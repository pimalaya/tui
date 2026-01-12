{
  description = "Collection of crossterm widgets shared accross Pimalaya projects";

  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs/staging-next";
    };
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pimalaya = {
      url = "github:pimalaya/nix";
      flake = false;
    };
  };

  outputs = inputs: (import inputs.pimalaya).mkFlakeOutputs inputs {
    shell = ./shell.nix;
  };
}

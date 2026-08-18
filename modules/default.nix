{
  config,
  lib,
  pkgs,
  self,
  inputs,
  ...
} : let
  inherit (lib) types mkOption mkEnableOption mkMerge;
    
in { 
    imports = [ ./../options ./zigduck.nix ./assertions.nix ./helpers.nix ];
      
   }

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          runtimeLibs = pkgs.lib.optionals pkgs.stdenv.isLinux (
            with pkgs;
            # See: <https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md#nix>
            [
              alsa-lib
              udev
              vulkan-loader
              libxkbcommon
              wayland
              libx11
              libxcursor
              libxi
              libxrandr
            ]
          );
        in
        {
          default = pkgs.mkShell {
            packages = [
              toolchain
              pkgs.pkg-config
              pkgs.trunk
              pkgs.binaryen
            ];
            buildInputs = runtimeLibs;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          };
        }
      );
    };
}

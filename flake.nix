{
  description = "fastsse development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              overlays = [(import rust-overlay)];
            };
          });
    in
    {
      devShells = forAllSystems ({ pkgs }:
        let
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = ["clippy" "rustfmt"];
            targets = ["wasm32-unknown-unknown"];
          };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              binaryen
              nodejs_24
              pnpm
              rust
              wasm-pack
            ];

            shellHook = ''
              export PATH="$PWD/node_modules/.bin:$PATH"
            '';
          };
        });

      formatter = forAllSystems ({ pkgs }: pkgs.nixpkgs-fmt);
    };
}

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
          rustExtensions = ["clippy" "rustfmt"];
          rustTargets = ["wasm32-unknown-unknown"];
          mkRust = channel: channel.default.override {
            extensions = rustExtensions;
            targets = rustTargets;
          };
          rustLatest = mkRust pkgs.rust-bin.stable.latest;
          rustMsrv = mkRust pkgs.rust-bin.stable."1.88.0";
          mkShell = rust: pkgs.mkShell {
            packages = with pkgs; [
              binaryen
              cargo-deny
              nodejs_24
              pnpm
              rust
              wasm-pack
            ];

            shellHook = ''
              export PATH="$PWD/node_modules/.bin:$PATH"
            '';
          };
        in
        {
          default = mkShell rustLatest;
          msrv = mkShell rustMsrv;
        });

      formatter = forAllSystems ({ pkgs }: pkgs.nixpkgs-fmt);
    };
}

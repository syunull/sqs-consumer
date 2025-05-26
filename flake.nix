{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2505.*.tar.gz";
    rust-overlay.url = "github:oxalica/rust-overlay/stable"; # A helper for Rust + Nix
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      overlays = [
        (import rust-overlay)
        (self: super: { rustToolchain = super.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml; })
      ];

      allSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs allSystems (system: f { pkgs = import nixpkgs { inherit overlays system; }; });
    in
    {
      devShells = forAllSystems (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages =
              (with pkgs; [
                bacon
                go-task
                pre-commit
                rustToolchain
              ])
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
                with pkgs;
                [
                  libiconv
                ]
              );
            shellHook = ''
              alias awslocal="docker exec -t localstack-cli aws"
            '';
          };
        }
      );
    };
}

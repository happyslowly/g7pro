{
  description = "Rust dev environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/26.05";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          clippy
          rust-analyzer
          rustfmt
          taplo

          pkg-config
          systemd
        ];

        shellHook = ''
          export MANPATH="${pkgs.lib.getMan pkgs.man-pages}/share/man:${pkgs.man-pages-posix}/share/man:''${MANPATH:-}"
        '';
      };
    };
}

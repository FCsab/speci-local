{
  description = "speci-local: offline ELTE specification editor";

  # flake.lock is bumped weekly by .github/workflows/flake-update.yml.
  # The package itself needs no manual updates: the version comes from Cargo.toml
  # and the crate hashes from Cargo.lock.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      lib = nixpkgs.lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      cargoToml = lib.importTOML ./Cargo.toml;

      # libraries winit / wgpu / glutin open at runtime with dlopen
      runtimeLibs =
        pkgs:
        lib.optionals pkgs.stdenv.hostPlatform.isLinux (
          with pkgs;
          [
            wayland
            libxkbcommon
            vulkan-loader
            libGL
            libx11
            libxcursor
            libxi
            libxrandr
          ]
        );
    in
    {
      packages = forAllSystems (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.speci-local;
        speci-local = pkgs.rustPlatform.buildRustPackage {
          pname = "speci-local";
          version = cargoToml.workspace.package.version;

          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./build.rs
              ./src
              ./assets
              ./tests
              ./android
            ];
          };
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.copyDesktopItems ];

          # the GUI tests need a display / GPU, which the build sandbox doesn't have
          doCheck = false;

          # rpath instead of a wrapper script: the app writes a .desktop entry pointing at
          # its own executable, which must keep working when started directly
          postFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            patchelf --add-rpath ${lib.makeLibraryPath (runtimeLibs pkgs)} $out/bin/speci-local
          '';

          postInstall = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            install -Dm644 assets/icon.png $out/share/icons/hicolor/256x256/apps/speci-local.png
          '';

          desktopItems = lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            (pkgs.makeDesktopItem {
              name = "speci-local";
              desktopName = "speci-local";
              comment = "Offline ELTE specification editor";
              exec = "speci-local %f";
              icon = "speci-local";
              startupWMClass = "speci-local";
              categories = [
                "Development"
                "Education"
              ];
            })
          ];

          meta = {
            description = "Offline ELTE specification editor";
            homepage = "https://github.com/FCsab/speci-local";
            mainProgram = "speci-local";
            platforms = systems;
          };
        };
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
          ];
          LD_LIBRARY_PATH = lib.makeLibraryPath (runtimeLibs pkgs);
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}

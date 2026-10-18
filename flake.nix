{
    inputs = {
        flake.url = "github:fenhl/flake";
        aurelia = {
            url = "github:Drackrath/Aurelia";
            inputs.nixpkgs.follows = "flake/nixpkgs";
        };
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "flake/nixpkgs";
        };
    };
    outputs = attrs: attrs.flake.lib {
        config.allowUnfree = true;
        overlays = [
            attrs.rust-overlay.overlays.default
        ];
        devShells = {
            default = { pkgs, ... }: let
                dotnetPkg = with pkgs.dotnetCorePackages; combinePackages [
                    runtime_10_0
                    sdk_10_0
                ];
            in {
                DOTNET_ROOT = "${dotnetPkg}/share/dotnet";
                packages = with pkgs; [
                    attrs.aurelia.packages.${pkgs.stdenv.hostPlatform.system}.default
                    cargo
                    dotnetPkg
                    mono
                ] ++ lib.optional (pkgs.stdenv.isLinux) steam-run;
            };
            pre-commit = { pkgs, ... }: {
                packages = with pkgs; [
                    rust-bin.nightly.latest.default # nightly cargo, required to run the pre-commit script
                ];
            };
        };
        packages = {
            raw = { pkgs, ... }: let
                manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
            in pkgs.rustPlatform.buildRustPackage {
                inherit (manifest) version;
                pname = "mod-om-raw";
                src = ./.;
                cargoLock = {
                    allowBuiltinFetchGit = true; # allows omitting cargoLock.outputHashes
                    lockFile = ./Cargo.lock;
                };
                postFixup = ''
                    mv $out/bin/mod-om $out/bin/mod-om-raw
                '';
            };
            default = { pkgs, system, ... }: let
                dotnetPkg = with pkgs.dotnetCorePackages; combinePackages [
                    runtime_10_0
                    sdk_10_0
                ];
                manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
            in pkgs.runCommand "mod-om-${manifest.version}" {
                inherit (manifest) version;
                pname = "mod-om";
                nativeBuildInputs = with pkgs; [
                    installShellFiles # required for `installShellCompletion` in postInstall hook
                    makeWrapper # required for wrapProgram in postFixup hook
                ];
            } (''
                makeWrapper ${attrs.self.packages.${system}.raw}/bin/mod-om-raw $out/bin/mod-om \
                    --prefix PATH : ${pkgs.lib.makeBinPath (with pkgs; [
                        attrs.aurelia.packages.${pkgs.stdenv.hostPlatform.system}.default
                        dotnetPkg
                        mono
                    ] ++ lib.optional (pkgs.stdenv.isLinux) steam-run)} \
                    --set DOTNET_ROOT ${dotnetPkg}/share/dotnet
            '' + (let
                mod-om = "${pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages} $out/bin/mod-om";
            in pkgs.lib.optionalString (pkgs.stdenv.hostPlatform.emulatorAvailable pkgs.buildPackages) ''
                installShellCompletion --cmd mod-om \
                    --bash <(COMPLETE=bash ${mod-om}) \
                    --fish <(COMPLETE=fish ${mod-om}) \
                    --zsh <(
                        COMPLETE=zsh ${mod-om}
                        # fix completion not working on first Tab press of shell session
                        echo '_clap_dynamic_completer_mod_om'
                    )
            ''));
        };
    };
}

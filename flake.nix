{
    inputs = {
        flake.url = "github:fenhl/flake";
        aurelia = {
            url = "github:Drackrath/Aurelia";
            inputs.nixpkgs.follows = "flake/nixpkgs";
        };
        mutatum-setup-kit = {
            url = "https://github.com/QuintessentialOM/OpusMutatum/releases/download/v1.0-CE/MutatumSetupKit_Beta0.4.zip";
            flake = false;
        };
        om-mappings = {
            url = "github:QuintessentialOM/om_mappings";
            flake = false;
        };
        opus-mutatum = {
            url = "github:QuintessentialOM/OpusMutatum";
            flake = false;
        };
    };
    outputs = attrs: attrs.flake.lib {
        config.allowUnfree = true;
        devShells.default = { pkgs, ... }: let
            dotnetPkg = with pkgs.dotnetCorePackages; combinePackages [
                runtime_10_0
                sdk_10_0
            ];
        in {
            DOTNET_ROOT = "${dotnetPkg}/share/dotnet";
            packages = with pkgs; [
                dotnetPkg
            ] ++ lib.optional (pkgs.stdenv.isLinux) steam-run;
        };
        packages.default = { pkgs, ... }: let
            dotnetPkg = with pkgs.dotnetCorePackages; combinePackages [
                runtime_10_0
                sdk_10_0
            ];
        in pkgs.writeShellApplication {
            name = "mod-om";
            runtimeInputs = with pkgs; [
                attrs.aurelia.packages.${pkgs.stdenv.hostPlatform.system}.default
                dotnetPkg
                mono
            ] ++ lib.optional (pkgs.stdenv.isLinux) steam-run;
            text = ''
                # based on https://discord.com/channels/278707932089155584/1522380759254433862/1551995330973671454

                # 1st argument must be a path that doesn't exist yet. A modded instance of OM will be created there
                if test -e "$1"; then
                    echo "$1 already exists, launching"
                else
                    # ensure community edition is active TODO interactively switch to community-edition if not already active
                    aurelia list --json | jq -e '.[] | select(.app_id == 558990).active_branch == "community-edition"'
                    # copy the game files to the new instance
                    cp -r "$(aurelia list --json | jq -r '.[] | select(.app_id == 558990).install_path')" "$1"

                    # 0. install the .net 10 sdk + runtime
                    # (handled by runtimeInputs)

                    # 1. clone <https://github.com/QuintessentialOM/OpusMutatum> + switch to the `coreifier` branch
                    # (handled by flake inputs)

                    # 2. run `dotnet publish -c:Debug`
                    cp -r ${attrs.opus-mutatum} "$1"/opus-mutatum
                    chmod -R +w "$1"/opus-mutatum
                    env -C "$1"/opus-mutatum dotnet publish -c:Debug

                    # 3. copy everything in the `bin/Debug/net10.0/publish` folder into your opus magnum root
                    cp -r "$1"/opus-mutatum/bin/Debug/net10.0/publish/* "$1"

                    # 4. clone <https://github.com/Fate-of-Adrastus/om_mappings> somewhere
                    # (handled by flake inputs)

                    # 5. copy everything in the `intermediary` and `named` folders into the corresponding folders in `<opus magnum root>/modded/Mappings`
                    mkdir -p "$1"/modded/Mappings
                    cp -r ${attrs.om-mappings}/intermediary "$1"/modded/Mappings/intermediary
                    cp -r ${attrs.om-mappings}/named "$1"/modded/Mappings/named
                fi

                # additional setup for Quintessential
                #TODO automatically use the repos' latest releases
                for file in ${attrs.mutatum-setup-kit}/modded/*.dll; do
                    ln -fs "$file" "$1"/modded
                done
                mkdir -p "$1"/modded/Mods
                cp ${pkgs.fetchurl {
                    url = "https://github.com/QuintessentialOM/Quintessential/releases/download/v0.8.0/QuintessentialCore_0.8.0.zip";
                    hash = "sha256:6b2cf2ef2dbfbd72a5956842b38de9982a715db9d20ace2d2d20063c0ceb5436";
                }} "$1"/modded/Mods/QuintessentialCore_0.8.0.zip
                cp ${pkgs.fetchurl {
                    url = "https://github.com/QuintessentialOM/Quintessential/releases/download/v0.8.0/QuintessentialData_0.8.0.zip";
                    hash = "sha256:3b1a72a99c6d61b99e066af965ad1c354a2eb1dbd931f106c077c252b676453b";
                }} "$1"/modded/Mods/QuintessentialData_0.8.0.zip
                cp ${pkgs.fetchurl {
                    url = "https://github.com/QuintessentialOM/Quintessential/releases/download/v0.8.0/QuintessentialUI_0.8.0.zip";
                    hash = "sha256:655c20ca8edf096725da775c6a39edaab5c1cf50806086b3cc331f3f69827961";
                }} "$1"/modded/Mods/QuintessentialUI_0.8.0.zip

                # 6. in your opus magnum root, run `./OpusMutatum`. you should now have a modded instance (without quintessential, because i havent figured out how to compile that yet
                export DOTNET_ROOT=${dotnetPkg}/share/dotnet
                ${if pkgs.stdenv.isLinux then ''
                    env -C "$1" steam-run ./OpusMutatum
                '' else ''
                    env -C "$1" ./OpusMutatum
                ''}
            '';
        };
    };
}

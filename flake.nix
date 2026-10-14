{
    inputs = {
        flake.url = "github:fenhl/flake";
        aurelia.url = "github:Drackrath/Aurelia";
        mutatum-setup-kit = {
            url = "https://github.com/Fate-of-Adrastus/OpusMutatum/releases/download/v0.3-alpha_SetupKit/MutatumSetupKit_Alpha0.3.zip";
            flake = false;
        };
        om-mappings = {
            url = "github:Fate-of-Adrastus/om_mappings";
            flake = false;
        };
        opus-mutatum = {
            url = "github:QuintessentialOM/OpusMutatum/dev";
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
                ln -fs ${pkgs.fetchurl {
                    url = "https://github.com/Fate-of-Adrastus/Quintessential/releases/download/v0.7.1-alpha_Quintessential/QuintessentialCore_0.7.1.zip";
                    hash = "sha256:7d3bf1db3434155208e729028f593911015bc4e8be34af313212d32e4be2b009";
                }} "$1"/modded/Mods/QuintessentialCore_0.7.1.zip
                ln -fs ${pkgs.fetchurl {
                    url = "https://github.com/Fate-of-Adrastus/Quintessential/releases/download/v0.7.1-alpha_Quintessential/QuintessentialData_0.7.1.zip";
                    hash = "sha256:eab6bcdd9eb7227a35f5246dce4c25aa07b4e671289f82c5dc85f949d9176310";
                }} "$1"/modded/Mods/QuintessentialData_0.7.1.zip
                ln -fs ${pkgs.fetchurl {
                    url = "https://github.com/Fate-of-Adrastus/Quintessential/releases/download/v0.7.1-alpha_Quintessential/QuintessentialUI_0.7.1.zip";
                    hash = "sha256:a155c80f7bfa2c7d2eab65eff88dcf7ae99764a0e02f1c3c928ba1f9b1091393";
                }} "$1"/modded/Mods/QuintessentialUI_0.7.1.zip

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

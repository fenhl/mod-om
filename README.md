This is a script that will set up a modded instance of [_Opus Magnum_](https://en.wikipedia.org/wiki/Opus_Magnum).

# Prerequisites

* macOS or Linux (for Windows, please follow [the manual setup instructions](https://github.com/Fate-of-Adrastus/Quintessential/wiki/How-to-mod-the-game#windows))
* [_Opus Magnum_](https://store.steampowered.com/app/558990/Opus_Magnum/) installed via [Steam](https://store.steampowered.com/about/)

# Installation

The script can be installed using the Nix package manager, which also automatically installs all dependencies for running the modded game. Alternatively, the script is also available as a standalone program, which will attempt to install these dependencies using an available package manager or fall back to asking the user to install them manually.

## Using Nix

1. Install Nix and ensure flakes are enabled:
    * On macOS, follow the [nix-darwin](https://github.com/nix-darwin/nix-darwin#prerequisites) prerequisites and install instructions using flakes
    * On NixOS, set `nix.settings.extra-experimental-features = [ "nix-command" "flakes" ];`
    * On non-NixOS Linux, you have multiple options:
        * Install using [the Lix installer](https://lix.systems/install/#on-any-other-linuxmacos-system)
        * Install using [the Determinate Nix installer](https://docs.determinate.systems/)
        * Install using your distribution's package manager or [the official installer](https://nixos.org/download/), then edit `nix.conf` to include `experimental-features = nix-command flakes`. The `nix.conf` file is in `/etc/nix` for multi-user installations, or `~/.config/nix` for single-user installations.
2. Optionally, to speed up installation:
    * On NixOS, add this to your system configuration:
        ```nix
        nix.settings = {
            extra-substituters = [ "https://fenhl.cachix.org" ];
            extra-trusted-public-keys = [ "fenhl.cachix.org-1:TTD8zELekSbEYQVNonmACllnTlvDdCSMgO9mqNwADPI=" ];
        };
        ```
    * On other operating systems, add this to your `nix.conf`:
        ```
        extra-substituters = https://fenhl.cachix.org
        extra-trusted-public-keys = fenhl.cachix.org-1:TTD8zELekSbEYQVNonmACllnTlvDdCSMgO9mqNwADPI=
        ```
        The `nix.conf` file is in `/etc/nix` for multi-user installations, or `~/.config/nix` for single-user installations.
3. You can now install and run the script in one command using `nix run github:fenhl/mod-om`. Alternatively, add the following to your NixOS system flake to be able to run it as `mod-om`:
    ```nix
    {
        inputs.mod-om.url = "github:fenhl/mod-om";
        outputs = { nixpkgs, mod-om, ... }: {
            nixosConfigurations.default = nixpkgs.lib.nixosSystem {
                modules = [
                    ({ pkgs, ... }: {
                        environment.systemPackages = [
                            mod-om.packages.${pkgs.stdenv.hostPlatform.system}.default
                        ];
                    })
                ];
            };
        };
    }
    ```

## Standalone

1. Download the script:
    * [for macOS](https://github.com/fenhl/mod-om/releases/latest/download/mod-om-aarch64-macos)
    * [for Linux (x86-64)](https://github.com/fenhl/mod-om/releases/latest/download/mod-om-x86_64-linux)
    * [for Linux (aarch64)](https://github.com/fenhl/mod-om/releases/latest/download/mod-om-aarch64-linux)
2. Open a command line (e.g. “Terminal” or “Konsole” app)
3. On the command line, navigate to the folder with the downloaded script in it, e.g.:
    ```
    cd Downloads
    ```
4. Make the script executable:
    ```
    chmod +x mod-om-*
    ```
5. You can now run the script as `./mod-om-*`

# Usage

1. Change to the Opus Magnum [Community Edition](https://omwiki.hoekri.nl/index.php/Community_Edition).
    1. In your Steam library, select Opus Magnum, click on Manage (*gear icon*).
    2. Select “Properties”.
    3. Select “Game Versions & Betas”, then select `community-edition`.
2. On the command line, run `mod-om init out`, replacing:
    * `mod-om` with the command given in the last step of the [Installation](#installation) subsection you followed
    * `out` with the path where the modded instance should be created. This directory should not exist yet.
3. The modded game will launch. After it finishes loading, confirm that you see “3 mods loaded” in the bottom left corner of the title screen, then exit the game.
4. You can now install some [mods](https://github.com/QuintessentialOM/Quintessential/wiki/Mod-List) or use the modded instance for [mod development](https://github.com/Fate-of-Adrastus/Quintessential/wiki/Getting-Started). When developing mods, it is recommended to use a separate instance for each mod.

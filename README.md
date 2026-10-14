This is a script that will set up a modded instance of [_Opus Magnum_](https://en.wikipedia.org/wiki/Opus_Magnum). It works on macOS and Linux (for Windows, please follow [the manual setup instructions](https://github.com/Fate-of-Adrastus/Quintessential/wiki/How-to-mod-the-game#windows)).

# Usage

1. Install Nix and ensure flakes are enabled:
    * On macOS, follow the [nix-darwin](https://github.com/nix-darwin/nix-darwin#prerequisites) prerequisites and install instructions using flakes
    * On NixOS, set `nix.settings.extra-experimental-features = [ "nix-command" "flakes" ];`
    * On non-NixOS Linux, you have multiple options:
        * Install using [the Lix installer](https://lix.systems/install/#on-any-other-linuxmacos-system)
        * Install using [the Determinate Nix installer](https://docs.determinate.systems/)
        * Install using your distribution's package manager or [the official installer](https://nixos.org/download/), then edit `nix.conf` to include `experimental-features = nix-command flakes`. The `nix.conf` file is in `/etc/nix` for multi-user installations, or `~/.config/nix` for single-user installations.
2. In a terminal, run `nix run github:fenhl/mod-om out`, replacing `out` with the path where the modded instance should be created. This directory should not exist yet.
    * You can also replace `out` with the path to an existing modded instance to update the toolchain.
3. The modded game will launch. After it finishes loading, confirm that you see “3 mods loaded” in the bottom left corner of the title screen, then exit the game.
4. You can now use the modded instance for [mod development](https://github.com/Fate-of-Adrastus/Quintessential/wiki/Getting-Started). It is recommended to use a separate instance for each mod.

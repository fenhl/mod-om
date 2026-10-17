This is a script that will set up a modded instance of [_Opus Magnum_](https://en.wikipedia.org/wiki/Opus_Magnum).

# Prerequisites

* macOS or Linux (for Windows, please follow [the manual setup instructions](https://github.com/Fate-of-Adrastus/Quintessential/wiki/How-to-mod-the-game#windows))
* [_Opus Magnum_](https://store.steampowered.com/app/558990/Opus_Magnum/) installed via [Steam](https://store.steampowered.com/about/)

# Usage

1. Install Nix and ensure flakes are enabled:
    * On macOS, follow the [nix-darwin](https://github.com/nix-darwin/nix-darwin#prerequisites) prerequisites and install instructions using flakes
    * On NixOS, set `nix.settings.extra-experimental-features = [ "nix-command" "flakes" ];`
    * On non-NixOS Linux, you have multiple options:
        * Install using [the Lix installer](https://lix.systems/install/#on-any-other-linuxmacos-system)
        * Install using [the Determinate Nix installer](https://docs.determinate.systems/)
        * Install using your distribution's package manager or [the official installer](https://nixos.org/download/), then edit `nix.conf` to include `experimental-features = nix-command flakes`. The `nix.conf` file is in `/etc/nix` for multi-user installations, or `~/.config/nix` for single-user installations.
2. Change to the [Community Edition](https://omwiki.hoekri.nl/index.php/Community_Edition).
    1. In your Steam library, select Opus Magnum, click on Manage (*gear icon*).
    2. Select “Properties”.
    3. Select “Game Versions & Betas”, then select `community-edition`.
3. In a terminal, run `nix run github:fenhl/mod-om init out`, replacing `out` with the path where the modded instance should be created. This directory should not exist yet.
    * You can also replace `out` with the path to an existing modded instance to update the toolchain.
4. The modded game will launch. After it finishes loading, confirm that you see “3 mods loaded” in the bottom left corner of the title screen, then exit the game.
5. You can now install some [mods](https://github.com/QuintessentialOM/Quintessential/wiki/Mod-List) or use the modded instance for [mod development](https://github.com/Fate-of-Adrastus/Quintessential/wiki/Getting-Started). When developing mods, it is recommended to use a separate instance for each mod.

# Yakuza 0, Kiwami, Kiwami 2 & Like A Dragon Free Camera Tool
![build-release](https://github.com/etra0/yakuza-freecam/workflows/build-release/badge.svg)

<a href='https://ko-fi.com/U7U81LC5Q' target='_blank'><img height='36' style='border:0px;height:36px;' src='https://cdn.ko-fi.com/cdn/kofi3.png?v=2' border='0' alt='Buy Me a Coffee at ko-fi.com' /></a>

<p align="center">
<img height=400 src="https://raw.githubusercontent.com/etra0/yakuza-freecam/master/assets/cover.png"/>
</p>

This is a free camera tool for Yakuza 0, Kiwami and Kiwami 2. It works in Cutscenes and freeroam.

[DEMO](https://twitter.com/etra0/status/1264050436031623169)

# This only works with the Steam version

## Features
Yakuza 0 & Kiwami:
- You can release the camera in almost every place
- You can pause the cinematics and move the camera around

Yakuza Kiwami 2:
- You can release the camera in almost every place
- You can pause in freeroam and in the cinematics (experimental)

Yakuza Like A Dragon:
- You can release the camera in almost every place
- You can change engine's speed at any time (i.e. pause the game).
- Check the [instructions](#usage-ylad) for this photomode before using it.

## Usage

You should see a Command Prompt window with instructions. If one briefly flashes on the screen, or doesn't appear at all, you may need to open Command Prompt yourself and run it to see what went wrong.

## Usage YLAD:
**You have to deactivate Motion Blur if you want to mess with the world paused**

Currently, you can only use it with a controller (no keyboard support)
- R1 + R3: Photo Mode Activation
- Left/Right arrow: Change engine speed
- L2/R2: Change FoV


## Compilation
Yakuza Zero:

```
cargo build -p yakuza0 --release
```

Yakuza Kiwami:

```
cargo build -p kiwami --release
```

Yakuza Kiwami 2:

```
cargo build -p kiwami2 --release
```

# Thanks

I want to give some special thanks to:
- [@Olivier__Cesar](https://twitter.com/Olivier__Cesar) for giving me some
  special support & sponsorship (seriously, thanks!)
- Every supporter at Ko-Fi. For real, thanks a lot!
- [@Miko_M2](https://twitter.com/Miko_M2) for the support in Ko-Fi and also the icon of the Kiwami 2 freecam.
- [Timo654](https://github.com/Timo654) for the support in Ko-Fi and also the icon of Yakuza 0 freecam.
- Galen#5628 for the icon of the Yakuza Like A Dragon freecam.
- [Yakuza Modding Community](https://discord.gg/7HCGNCYp3V) for being a cool
  community and giving lots of support and testing.

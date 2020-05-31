# Yakuza 0, Kiwami & Kiwami 2 Free Camera Tool
![build-release](https://github.com/etra0/yakuza-freecam/workflows/build-release/badge.svg)

<a href='https://ko-fi.com/U7U81LC5Q' target='_blank'><img height='36' style='border:0px;height:36px;' src='https://cdn.ko-fi.com/cdn/kofi3.png?v=2' border='0' alt='Buy Me a Coffee at ko-fi.com' /></a>

![Kiryu](https://i.imgur.com/s9Od0q4.jpg)

This is a revamped and rewritten camera tool for the Yakuza 0, Kiwami and Kiwami 2. It works in Cutscenes and freeroam.

# This only works with the Steam version

## Features
Yakuza 0 & Kiwami:
- You can release the camera in almost every place
- You can pause the cinematics and move the camera around

Yakuza Kiwami 2:
- You can release the camera in almost every place
- You can pause in freeroam and in the cinematics (experimental)

## Usage

You should see a Command Prompt window with instructions. If one briefly flashes on the screen, or doesn't appear at all, you may need to open Command Prompt yourself and run it to see what went wrong.

## Compilation
Yakuza Zero:

```
cargo build --bin yakuza-freecam --release
```

Yakuza Kiwami:

```
cargo build --bin kiwami-freecam --features kiwami --release
```

Yakuza Kiwami 2:

```
cargo build --bin kiwami2-freecam --features kiwami2 --release
```



all:
	cargo.exe +nightly build --bin yakuza0-freecam --release
	cargo.exe +nightly build --bin kiwami-freecam --features kiwami --release
	cargo.exe +nightly build --bin kiwami2-freecam --features kiwami2 --release

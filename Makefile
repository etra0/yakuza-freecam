all:
	cargo.exe build --bin yakuza0-freecam --release
	cargo.exe build --bin kiwami-freecam --features kiwami --release
	cargo.exe build --bin kiwami2-freecam --features kiwami2 --release

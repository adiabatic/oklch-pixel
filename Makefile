test:
	# the fallback here looks obviously grayer than anything close to #008080
	cargo run -- 0.6893 0.1554 200.51

release:
	cargo build --release

debug:
	cargo build

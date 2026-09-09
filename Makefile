.PHONY: test
#Test the project and print output to the terminal.
test:
	STD_PATH=./lib/std cargo test --verbose

#Run test with Rust backtraces enabled.
#Shows a stack trace when a test panics.
test-backtrace:
	RUST_BACKTRACE=1 cargo test -- --no-capture

#Run test, display all output, and save it to a file.
test-logging:
	@test -n "$(FILE)" || (echo "Use with: make test-logging FILE=file.slx"; exit 1)
	cargo test -- --no-capture --show-output 2>&1 | tee $(FILE)

#Checks if the compiler is running properly, according to CI/CD
check:
	STD_PATH=./lib/std cargo test --verbose
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo build --verbose

install_std:
	mkdir -p ~/.slynx/std
	cp -r lib/std/* ~/.slynx/std

uninstall_std:
	rm -r ~/.slynx/std

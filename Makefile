#Test the project and print output to the terminal.
test:
	cargo test -- --no-capture

#Run test with Rust backtraces enabled.
#Shows a stack trace when a test panics.
test-backtrace:
	RUST_BACKTRACE=1 cargo test -- --no-capture

#Run test, display all output, and save it to a file.
test-logging:
	@test -n "$(FILE)" || (echo "Use with: make test-logging FILE=file.slx"; exit 1)
	cargo test -- --no-capture --show-output 2>&1 | tee $(FILE)

#Run the application in debug mode with backtraces enabled targeting a specific file.
#Useful for diagnosing runtime panics.
debug:
	@test -n "$(FILE)" || (echo "Use with: make debug FILE=file.slx"; exit 1)
	RUST_BACKTRACE=1 cargo run -- --target $(FILE)

#Run the application in debug mode targeting a specific file.
run:
	@test -n "$(FILE)" || (echo "Use with: make run FILE=file.slx"; exit 1)
	cargo run -- --target $(FILE)

#Run the application in release mode.
release:
	cargo run --release

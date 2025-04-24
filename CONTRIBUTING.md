# Contributing to DevRS

First off, thank you for considering contributing to DevRS! We welcome contributions from everyone. Following these guidelines helps communicate that you respect the time of the developers managing and developing this open-source project. In return, they should reciprocate that respect in addressing your issue, assessing changes, and helping you finalize your pull requests.

## Code of Conduct

This project and everyone participating in it are governed by the [DevRS Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers (refer to the Code of Conduct for contact methods).

## How Can I Contribute?

There are many ways to contribute, from writing tutorials or blog posts, improving the documentation, submitting bug reports and feature requests, or writing code which can be incorporated into DevRS itself.

### Reporting Bugs

* **Ensure the bug was not already reported** by searching on GitHub under [Issues](https://github.com/christimahu/devrs/issues). * If you're unable to find an open issue addressing the problem, [open a new one](https://github.com/christimahu/devrs/issues/new). Be sure to include a **title and clear description**, as much relevant information as possible, and a **code sample or an executable test case** demonstrating the expected behavior that is not occurring.
* Describe the **expected behavior** and the **actual behavior**.
* Provide **steps to reproduce** the issue.
* Include details about your **operating system, DevRS version, Docker version, and Rust version**.

### Suggesting Enhancements

* Open a new issue, clearly marking it as a feature request or enhancement.
* Explain the **motivation** for the change. What problem does it solve? Why is this enhancement useful?
* Provide a **step-by-step description** of the suggested enhancement in as many details as possible.
* Provide **examples** of how the enhancement would be used.

### Pull Requests

Pull requests are the best way to propose changes to the codebase. We actively welcome your pull requests:

1.  **Fork the repo** and create your branch from `main`.
2.  If you've added code that should be tested, **add tests**.
3.  If you've changed APIs, **update the documentation** (README.md, command help text).
4.  Ensure the test suite passes (`cargo test --workspace`).
5.  Make sure your code lints (`cargo clippy --workspace -- -D warnings`).
6.  Format your code (`cargo fmt --all`).
7.  **Issue that pull request!**

#### Development Setup

* Clone the repository: `git clone <your-fork-url> ~/tools/devrs` (or your preferred location).
* Navigate to the directory: `cd ~/tools/devrs`.
* Build the project: `cargo build --workspace`.
* Run tests: `cargo test --workspace`.
* Run the CLI: `./target/debug/devrs <command> ...`.
* Consider using `devrs env shell` (after building the core env image) to work on DevRS itself within its own managed environment.

#### Pull Request Process

1.  Ensure any install or build dependencies are removed before the end of the layer when doing a build (if modifying Dockerfile).
2.  Update the README.md with details of changes to the interface, this includes new commands, options, environment variables, exposed ports, useful file locations, and container parameters.
3.  Increase the version numbers in `cli/Cargo.toml` and any relevant examples or documentation according to [SemVer](http://semver.org/).
4.  You may merge the Pull Request in once you have the sign-off of one other developer, or if you do not have permission to do that, you may request the second reviewer to merge it for you.

## Code Style

* Please follow the standard Rust style guidelines enforced by `rustfmt`.
* Run `cargo fmt --all` before committing to ensure consistent formatting.
* Run `cargo clippy --workspace -- -D warnings` to catch common mistakes and ensure code quality. Address clippy warnings.

Thank you for contributing!


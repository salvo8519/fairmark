# Installation

Fairmark requires Python 3.12 or newer
([supported versions](../project/dependencies.md#supported-python-versions)). Prebuilt wheels are published for Linux on x86_64 and
arm64 (glibc and musl), macOS on Intel and Apple silicon, and Windows on x86_64, so no Rust
compiler is needed. On other platforms pip builds the package from source, which needs a Rust
toolchain.

=== "uv"

    ```bash
    uv add fairmark                 # library and function-calling tools
    uv add "fairmark[mcp]"          # plus the MCP server
    ```

=== "pip"

    ```bash
    pip install fairmark
    pip install "fairmark[mcp]"
    ```

=== "CLI only (Rust)"

    ```bash
    cargo install fairmark-cli      # installs the `fairmark` binary
    ```

Check the installation:

```bash
python -c "import fairmark; print(fairmark.__version__)"
```

## From source

```bash
git clone https://github.com/salvo8519/fairmark
cd fairmark
uv sync --all-extras --all-groups   # builds the Rust extension with maturin
```

See [Contributing](../project/contributing.md) for the full development setup.

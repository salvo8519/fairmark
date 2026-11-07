# Installation

Fairmark requires Python 3.10 or newer. Prebuilt wheels are published for Linux, macOS and
Windows on x86_64 and arm64, so no Rust compiler is needed.

=== "pip"

    ```bash
    pip install fairmark            # library and function-calling tools
    pip install "fairmark[mcp]"     # plus the MCP server
    ```

=== "uv"

    ```bash
    uv add fairmark
    uv add "fairmark[mcp]"
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
git clone https://github.com/fairmark-dev/fairmark
cd fairmark
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev,mcp,docs]"    # builds the Rust extension with maturin
```

See [Contributing](../project/contributing.md) for the full development setup.

---
source: getting-started/installation.md
source_hash: 97f4e83fc9a9
---
# Installazione

Fairmark richiede Python 3.12 o successivo ([versioni
supportate](https://salvo8519.github.io/fairmark/project/dependencies/#supported-python-versions),
in inglese). Le wheel precompilate sono pubblicate per Linux su x86_64 e arm64 (glibc e musl),
macOS su Intel e Apple silicon e Windows su x86_64: non serve il compilatore Rust. Sulle altre piattaforme pip compila il pacchetto dai sorgenti, e serve
quindi il toolchain Rust.

=== "uv"

    ```bash
    uv add fairmark                 # libreria e tool per il function calling
    uv add "fairmark[mcp]"          # più il server MCP
    ```

=== "pip"

    ```bash
    pip install fairmark
    pip install "fairmark[mcp]"
    ```

=== "Solo CLI (Rust)"

    ```bash
    cargo install fairmark-cli      # installa il binario `fairmark`
    ```

Verifica l'installazione:

```bash
python -c "import fairmark; print(fairmark.__version__)"
```

## Dal codice sorgente

```bash
git clone https://github.com/salvo8519/fairmark
cd fairmark
uv sync --all-extras --all-groups   # compila l'estensione Rust con maturin
```

La configurazione completa dell'ambiente di sviluppo è descritta in
[Contributing](https://salvo8519.github.io/fairmark/project/contributing/) (in inglese).

---
source: getting-started/installation.md
source_hash: 7100fece51f3
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
    cargo install fairmark-cli --version 0.1.0-alpha.1 --locked   # installa il binario `fairmark`
    ```

!!! note "Versioni alpha"
    Le versioni pubblicate finora sono alpha. Finché non esiste una versione stabile, `uv add` e
    `pip install` installano l'ultima alpha. Per avere risultati riproducibili fissa la versione,
    per esempio `pip install "fairmark==0.1.0a1"`: una alpha può ancora cambiare input e
    risultati. `cargo install` richiede comunque la versione, come sopra.

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

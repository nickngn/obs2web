# obs2web

`obs2web` is a command-line tool that converts your [Obsidian](https://obsidian.md/) vault into a static website. It processes your Markdown files, preserves the folder structure, and renders a simple, clean website that you can host anywhere.

## Features

*   **Markdown to HTML:** Converts your Obsidian notes from Markdown to HTML.
*   **Preserves Structure:** Maintains your vault's folder and file structure.
*   **Static Site:** The output is a fully static website, which is fast, secure, and easy to host.

## Usage

To use `obs2web`, you need to provide the path to your Obsidian vault and the desired output directory.

```bash
obs2web --vault-path /path/to/your/vault --output-dir /path/to/your/output
```

### Arguments

*   `--vault-path` (`-v`): The path to your Obsidian vault.
*   `--output-dir` (`-o`): The directory where the static website will be generated.

## Installation

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/nickngn/obs2web.git
    cd obs2web
    ```
2.  **Build the project:**
    ```bash
    cargo build --release
    ```
3.  **Run the executable:**
    The executable will be located in the `target/release` directory.
    ```bash
    ./target/release/obs2web --vault-path /path/to/your/vault --output-dir /path/to/your/output
    ```

## License

This project is licensed under the MIT License.

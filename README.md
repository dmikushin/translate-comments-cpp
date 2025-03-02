# Translate comments in source code

Get source code comments to speak your language!

Comments are parsed using the power of [Tree Sitter](https://tree-sitter.github.io/tree-sitter/#available-parsers). Caching is built in to speed up processing new and edited docs.


## Building

```sh
git clone --recurse-submodules https://github.com/dmikushin/translate-comments-cpp.git
cd translate-comments-cpp
cargo install --path .
```


## Usage

If only the input filename is given, the translation is printed out as JSON:

```sh
cat test.cpp
/**<+回転翼(トリム付)+*/
translate-comments-cpp translate --input test.cpp -l "en_US"
[{"text":"/**<+回転翼(トリム付)+*/","text_translation":"/**<+Rotary Wing (with Trim)+*/","text_checksum":7513991081016594172}]
```

If the output filename is given as well (can be the same file as input), then the entire input source file with translated comments is saved into it:

```sh
cat test.cpp
/**<+回転翼(トリム付)+*/
translate-comments-cpp translate --input test.cpp -l "en_US" --output test_translated.cpp
cat test_translated.cpp 
/**<+Rotary Wing (with Trim)+*/
```


## Development

```sh
git clone --recurse-submodules https://github.com/dmikushin/translate-comments-cpp.git
cd translate-comments-cpp
cargo build
cargo check
```


trunk := home_directory() / ".cargo/bin/trunk"
slangc := "/opt/shader-slang-bin/bin/slangc"

shaders:
    mkdir -p gen
    for shader in splatfit backdrop; do \
        {{slangc}} src/shaders/$shader.slang -I src/shaders -target wgsl -o gen/$shader.wgsl; \
    done

shell:
    cargo run --quiet --bin shell > gen/shell.html
    python3 -c "import pathlib; t=pathlib.Path('index.template.html').read_text(); s=pathlib.Path('gen/shell.html').read_text(); pathlib.Path('index.html').write_text(t.replace('<!--shell-->', s))"

build: shaders shell
    {{trunk}} build --release --public-url ./

serve: shaders shell
    {{trunk}} serve --release

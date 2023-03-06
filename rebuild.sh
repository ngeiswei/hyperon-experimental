# Prepare environment
rustup update stable
pip install -U pip
cargo install --force cbindgen
pip install conan==1.57
conan profile new --detect default
pip install -e ./python[dev]

# Build Hyperon library
cd ./lib
cargo clean
cargo build
cargo test
cargo doc --no-deps
cd ..

# Build C and Python API
trash build; mkdir build; cd build
cmake ..
make
make check
cd ..

# Prepare environment
rustup update stable
pip install -U pip
cargo install --force cbindgen
pip install conan==1.60.1
conan profile new --detect default
pip install pip==23.1.2

# Build Hyperon library
cd ./lib
cargo clean
cargo build
cargo test
cargo doc --no-deps
cd ..

# Build C and Python API
trash build; mkdir build; cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
make -j4
make check
cd ..

# Install python library and executables
pip install -e ./python[dev]

# Test
cd python
pytest ./tests
cd ..

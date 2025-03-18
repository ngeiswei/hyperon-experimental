# You may need to create a Python environment first (this only needs
# to be done once)
#
# python -m venv hyperon-env
#
# Then source the environment before calling that script
#
# source hyperon-env/bin/activate
#
# Tip: create symbolic links pointing to the hyperon environment to be
# easier to be sourced from various places
#
# ln -s /path/to/venv/bin/activate activate
#
# That way you only need to call
#
# source activate

# Prepare environment
rustup update stable
pip install -U pip
cargo install --force cbindgen
pip install conan==2.13.0
conan profile detect --force
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
make -j16
make check
cd ..

# Install python library and executables
pip install -e ./python[dev]

# Test
cd python
pytest ./tests
cd ..

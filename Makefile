# ZundaLink Installer - Cross-platform Build Makefile
# Supports Windows, macOS, and Linux builds from any host platform

# Detect OS
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    MKDIR_P := mkdir
    RM_RF := rmdir /s /q
    CP := copy
    PATHSEP := \
else
    DETECTED_OS := $(shell sh -c 'uname 2>/dev/null || echo Unknown')
    MKDIR_P := mkdir -p
    RM_RF := rm -rf
    CP := cp
    PATHSEP := /
endif

# Project configuration
PROJECT_NAME := zundalink-installer
BUILD_DIR := build

# Target platforms
TARGET_WINDOWS := x86_64-pc-windows-msvc
TARGET_WINDOWS_GNU := x86_64-pc-windows-gnu
TARGET_MACOS_INTEL := x86_64-apple-darwin
TARGET_MACOS_ARM := aarch64-apple-darwin
TARGET_LINUX := x86_64-unknown-linux-gnu
TARGET_LINUX_MUSL := x86_64-unknown-linux-musl

# Output binary names
BINARY_WINDOWS := $(PROJECT_NAME)-windows-x64.exe
BINARY_MACOS_INTEL := $(PROJECT_NAME)-macos-intel
BINARY_MACOS_ARM := $(PROJECT_NAME)-macos-arm64
BINARY_MACOS_UNIVERSAL := $(PROJECT_NAME)-macos-universal
BINARY_LINUX := $(PROJECT_NAME)-linux-x64

# Default target: build all platforms available on current host
.PHONY: all
all: build-all

# Create build directory
$(BUILD_DIR):
ifeq ($(DETECTED_OS),Windows)
	-@if not exist "$(BUILD_DIR)" $(MKDIR_P) "$(BUILD_DIR)"
else
	@$(MKDIR_P) $(BUILD_DIR)
endif

# Detect host platform and build what's possible
.PHONY: build-all
ifeq ($(DETECTED_OS),Windows)
build-all: windows cross-linux
	@echo "========================================"
	@echo "Windows host build complete"
	@echo "Built: Windows (native), Linux (cross)"
	@echo "========================================"
else ifeq ($(DETECTED_OS),Darwin)
build-all: macos cross-linux
	@echo "========================================"
	@echo "macOS host build complete"
	@echo "Built: macOS (native), Linux (cross)"
	@echo "========================================"
else ifeq ($(DETECTED_OS),Linux)
build-all: linux cross-windows
	@echo "========================================"
	@echo "Linux host build complete"
	@echo "Built: Linux (native), Windows (cross)"
	@echo "========================================"
else
build-all: cross-all
	@echo "Unknown host OS - used cross-compilation for all platforms"
endif

# Build for current/native platform only
.PHONY: native build-native
native: build-native

ifeq ($(OS),Windows_NT)
build-native:
	@echo "Building for Windows (native)..."
	@rustup target add $(TARGET_WINDOWS) || true
	set RUSTFLAGS=-C target-feature=+crt-static && cargo build --release --target $(TARGET_WINDOWS)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_WINDOWS)\release\$(PROJECT_NAME).exe $(BUILD_DIR)\$(BINARY_WINDOWS)
else
	@cp target/$(TARGET_WINDOWS)/release/$(PROJECT_NAME).exe $(BUILD_DIR)/$(BINARY_WINDOWS)
endif
	@echo "✓ Windows native build complete: $(BUILD_DIR)/$(BINARY_WINDOWS)"
else ifeq ($(shell uname -s),Darwin)
build-native: macos
else ifeq ($(shell uname -s),Linux)
build-native:
	@echo "Building for Linux (native)..."
	@rustup target add $(TARGET_LINUX) 2>/dev/null || true
	cargo build --release --target $(TARGET_LINUX)
	@cp target/$(TARGET_LINUX)/release/$(PROJECT_NAME) $(BUILD_DIR)/$(BINARY_LINUX)
	@chmod +x $(BUILD_DIR)/$(BINARY_LINUX)
	@echo "✓ Linux native build complete: $(BUILD_DIR)/$(BINARY_LINUX)"
else
build-native:
	@echo "Unknown host OS - cannot determine native build target"
	@exit 1
endif

# ============================================
# Native Builds (build on same OS)
# ============================================

# Windows native build (run on Windows)
.PHONY: windows
windows: $(BUILD_DIR)/$(BINARY_WINDOWS)

$(BUILD_DIR)/$(BINARY_WINDOWS): $(BUILD_DIR)
	@echo "Building for Windows (x86_64)..."
	@rustup target add $(TARGET_WINDOWS) || true
	set RUSTFLAGS=-C target-feature=+crt-static && cargo build --release --target $(TARGET_WINDOWS)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_WINDOWS)\release\$(PROJECT_NAME).exe $(BUILD_DIR)\$(BINARY_WINDOWS)
else
	@cp target/$(TARGET_WINDOWS)/release/$(PROJECT_NAME).exe $(BUILD_DIR)/$(BINARY_WINDOWS)
endif
	@echo "✓ Windows build complete: $(BUILD_DIR)/$(BINARY_WINDOWS)"

# macOS builds (run on macOS)
.PHONY: macos
macos: macos-universal

.PHONY: macos-intel
macos-intel: $(BUILD_DIR)/$(BINARY_MACOS_INTEL)

$(BUILD_DIR)/$(BINARY_MACOS_INTEL): $(BUILD_DIR)
	@echo "Building for macOS (Intel x86_64)..."
	@rustup target add $(TARGET_MACOS_INTEL) 2>/dev/null || true
	cargo build --release --target $(TARGET_MACOS_INTEL)
	@cp target/$(TARGET_MACOS_INTEL)/release/$(PROJECT_NAME) $@
	@echo "✓ macOS Intel build complete: $@"

.PHONY: macos-arm
macos-arm: $(BUILD_DIR)/$(BINARY_MACOS_ARM)

$(BUILD_DIR)/$(BINARY_MACOS_ARM): $(BUILD_DIR)
	@echo "Building for macOS (ARM64)..."
	@rustup target add $(TARGET_MACOS_ARM) 2>/dev/null || true
	cargo build --release --target $(TARGET_MACOS_ARM)
	@cp target/$(TARGET_MACOS_ARM)/release/$(PROJECT_NAME) $@
	@echo "✓ macOS ARM64 build complete: $@"

.PHONY: macos-universal
macos-universal: $(BUILD_DIR)/$(BINARY_MACOS_UNIVERSAL)

$(BUILD_DIR)/$(BINARY_MACOS_UNIVERSAL): $(BUILD_DIR)/$(BINARY_MACOS_INTEL) $(BUILD_DIR)/$(BINARY_MACOS_ARM)
	@echo "Creating macOS universal binary..."
	@lipo -create -output $@ $(BUILD_DIR)/$(BINARY_MACOS_INTEL) $(BUILD_DIR)/$(BINARY_MACOS_ARM)
	@echo "✓ macOS universal build complete: $@"

# Linux native build (run on Linux)
.PHONY: linux
linux: $(BUILD_DIR)/$(BINARY_LINUX)

$(BUILD_DIR)/$(BINARY_LINUX): $(BUILD_DIR)
	@echo "Building for Linux (x86_64)..."
	@rustup target add $(TARGET_LINUX) 2>/dev/null || true
	cargo build --release --target $(TARGET_LINUX)
	@cp target/$(TARGET_LINUX)/release/$(PROJECT_NAME) $@
	@chmod +x $@
	@echo "✓ Linux build complete: $@"

# ============================================
# Cross-compilation (using cross tool)
# ============================================

# Check and install cross if needed
.PHONY: ensure-cross
ensure-cross:
	@which cross > /dev/null 2>&1 || (echo "Installing cross..." && cargo install cross --git https://github.com/cross-rs/cross)

# Cross-compile for Windows (from Linux/macOS)
.PHONY: cross-windows
cross-windows: ensure-cross $(BUILD_DIR)
	@echo "Cross-compiling for Windows (x86_64)..."
	@rustup target add $(TARGET_WINDOWS_GNU) 2>/dev/null || true
	cross build --release --target $(TARGET_WINDOWS_GNU)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_WINDOWS_GNU)\release\$(PROJECT_NAME).exe $(BUILD_DIR)\$(BINARY_WINDOWS)
else
	@cp target/$(TARGET_WINDOWS_GNU)/release/$(PROJECT_NAME).exe $(BUILD_DIR)/$(BINARY_WINDOWS)
endif
	@echo "✓ Windows cross-compile complete"

# Cross-compile for Linux (from Windows/macOS)
.PHONY: cross-linux
cross-linux: ensure-cross $(BUILD_DIR)
	@echo "Cross-compiling for Linux (x86_64)..."
	@rustup target add $(TARGET_LINUX) 2>/dev/null || true
	cross build --release --target $(TARGET_LINUX)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_LINUX)\release\$(PROJECT_NAME) $(BUILD_DIR)\$(BINARY_LINUX)
else
	@cp target/$(TARGET_LINUX)/release/$(PROJECT_NAME) $(BUILD_DIR)/$(BINARY_LINUX)
	@chmod +x $(BUILD_DIR)/$(BINARY_LINUX)
endif
	@echo "✓ Linux cross-compile complete"

# Cross-compile for Linux with musl (static linking)
.PHONY: cross-linux-musl
cross-linux-musl: ensure-cross $(BUILD_DIR)
	@echo "Cross-compiling for Linux (x86_64, musl static)..."
	@rustup target add $(TARGET_LINUX_MUSL) 2>/dev/null || true
	cross build --release --target $(TARGET_LINUX_MUSL)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_LINUX_MUSL)\release\$(PROJECT_NAME) $(BUILD_DIR)\$(PROJECT_NAME)-linux-x64-static
else
	@cp target/$(TARGET_LINUX_MUSL)/release/$(PROJECT_NAME) $(BUILD_DIR)/$(PROJECT_NAME)-linux-x64-static
	@chmod +x $(BUILD_DIR)/$(PROJECT_NAME)-linux-x64-static
endif
	@echo "✓ Linux musl cross-compile complete"

# Cross-compile for macOS Intel (from Linux/Windows)
.PHONY: cross-macos-intel
cross-macos-intel: ensure-cross $(BUILD_DIR)
	@echo "Cross-compiling for macOS (Intel x86_64)..."
	@rustup target add $(TARGET_MACOS_INTEL) 2>/dev/null || true
	cross build --release --target $(TARGET_MACOS_INTEL)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_MACOS_INTEL)\release\$(PROJECT_NAME) $(BUILD_DIR)\$(BINARY_MACOS_INTEL)
else
	@cp target/$(TARGET_MACOS_INTEL)/release/$(PROJECT_NAME) $(BUILD_DIR)/$(BINARY_MACOS_INTEL)
	@chmod +x $(BUILD_DIR)/$(BINARY_MACOS_INTEL)
endif
	@echo "✓ macOS Intel cross-compile complete"

# Cross-compile for macOS ARM (from Linux/Windows)
.PHONY: cross-macos-arm
cross-macos-arm: ensure-cross $(BUILD_DIR)
	@echo "Cross-compiling for macOS (ARM64)..."
	@rustup target add $(TARGET_MACOS_ARM) 2>/dev/null || true
	cross build --release --target $(TARGET_MACOS_ARM)
ifeq ($(DETECTED_OS),Windows)
	@copy target\$(TARGET_MACOS_ARM)\release\$(PROJECT_NAME) $(BUILD_DIR)\$(BINARY_MACOS_ARM)
else
	@cp target/$(TARGET_MACOS_ARM)/release/$(PROJECT_NAME) $(BUILD_DIR)/$(BINARY_MACOS_ARM)
	@chmod +x $(BUILD_DIR)/$(BINARY_MACOS_ARM)
endif
	@echo "✓ macOS ARM cross-compile complete"

# Build all using cross
.PHONY: cross-all
cross-all: cross-windows cross-linux cross-macos-intel cross-macos-arm
	@echo "========================================"
	@echo "All cross-compilation builds complete"
	@echo "========================================"

# ============================================
# Utility targets
# ============================================

# Clean build artifacts
.PHONY: clean
clean:
ifeq ($(DETECTED_OS),Windows)
	-cargo clean
	-if exist "$(BUILD_DIR)" $(RM_RF) "$(BUILD_DIR)"
else
	@cargo clean
	@$(RM_RF) $(BUILD_DIR)
endif
	@echo "Cleaned build artifacts"

# Install dependencies
.PHONY: setup
setup:
	@echo "Installing Rust targets..."
	@rustup target add $(TARGET_WINDOWS) || true
	@rustup target add $(TARGET_WINDOWS_GNU) || true
	@rustup target add $(TARGET_LINUX) || true
	@rustup target add $(TARGET_LINUX_MUSL) || true
	@rustup target add $(TARGET_MACOS_INTEL) || true
	@rustup target add $(TARGET_MACOS_ARM) || true
	@echo "Installing cross compilation tool..."
	@cargo install cross --git https://github.com/cross-rs/cross || true
	@echo "Setup complete"

# Generate checksums for all binaries
.PHONY: checksums
checksums: $(BUILD_DIR)
ifeq ($(DETECTED_OS),Windows)
	@echo "Checksums not available on Windows without additional tools"
else
	@cd $(BUILD_DIR) && sha256sum * > checksums.txt
	@echo "✓ Checksums written to $(BUILD_DIR)/checksums.txt"
endif

# Show help
.PHONY: help
help:
	@echo "ZundaLink Installer - Cross-platform Build System"
	@echo "Detected OS: $(DETECTED_OS)"
	@echo ""
	@echo "Main targets:"
	@echo "  make build-all        - Build all possible platforms on current host"
	@echo "  make all              - Same as build-all"
	@echo "  make build-native     - Build for current/native platform only"
	@echo "  make native           - Same as build-native"
	@echo ""
	@echo "Native builds (run on same OS):"
	@echo "  make windows          - Build for Windows x64 (run on Windows)"
	@echo "  make macos            - Build all macOS variants (run on macOS)"
	@echo "  make macos-intel      - Build for macOS Intel x86_64"
	@echo "  make macos-arm        - Build for macOS ARM64"
	@echo "  make macos-universal  - Create universal macOS binary"
	@echo "  make linux            - Build for Linux x64 (run on Linux)"
	@echo ""
	@echo "Cross-compilation targets (requires cross tool + Docker):"
	@echo "  make cross-all        - Build all platforms using cross"
	@echo "  make cross-windows    - Cross-compile for Windows (from Linux/macOS)"
	@echo "  make cross-linux      - Cross-compile for Linux (from Windows/macOS)"
	@echo "  make cross-linux-musl - Cross-compile for Linux with static linking"
	@echo "  make cross-macos-intel- Cross-compile for macOS Intel"
	@echo "  make cross-macos-arm  - Cross-compile for macOS ARM"
	@echo ""
	@echo "Utility targets:"
	@echo "  make setup            - Install required Rust targets and tools"
	@echo "  make clean            - Clean build artifacts"
	@echo "  make checksums        - Generate SHA256 checksums for binaries"
	@echo "  make help             - Show this help message"

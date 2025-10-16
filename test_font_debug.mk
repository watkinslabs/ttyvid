# Makefile for debugging font rendering with test_tight_font.cast
# This test validates character-to-glyph mapping with extensive logging

# Configuration
CAST_INPUT = test_output/test_tight_font.cast
FONT_FILE = test_output/converted_fonts/adwaita_mono_16.fd
OUTPUT_DIR = test_output
LOG_FILE = $(OUTPUT_DIR)/font_debug.log
OUTPUT_GIF = $(OUTPUT_DIR)/test_debug.gif

# Build configuration
CARGO = cargo
CARGO_FLAGS = build --release --no-default-features
BINARY = ./target/release/ttyvid

# Rendering parameters
FPS = 15
THEME = default

# Ensure output directory exists
$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

# Build the binary
.PHONY: build
build:
	$(CARGO) $(CARGO_FLAGS)
	@echo "Build complete: $(BINARY)"

# Run the test with debug logging
.PHONY: test
test: build $(OUTPUT_DIR)
	@echo "==== FONT RENDERING DEBUG TEST ===="
	@echo "Input:  $(CAST_INPUT)"
	@echo "Font:   $(FONT_FILE)"
	@echo "Output: $(OUTPUT_GIF)"
	@echo "Log:    $(LOG_FILE)"
	@echo ""
	@# Remove old log and output if they exist
	@rm -f $(LOG_FILE) $(OUTPUT_GIF)
	@# Run ttyvid with debug font logging enabled
	$(BINARY) convert \
		--input $(CAST_INPUT) \
		--output $(OUTPUT_GIF) \
		--font-file $(FONT_FILE) \
		--fps $(FPS) \
		--theme $(THEME) \
		--trailer \
		--debug-font
	@echo ""
	@echo "==== TEST COMPLETE ===="
	@if [ -f $(OUTPUT_GIF) ]; then \
		echo "✓ Output GIF created: $(OUTPUT_GIF)"; \
		ls -lh $(OUTPUT_GIF); \
	else \
		echo "✗ Output GIF NOT created!"; \
		exit 1; \
	fi
	@if [ -f $(LOG_FILE) ]; then \
		echo "✓ Debug log created: $(LOG_FILE)"; \
		echo "  Log has $$(wc -l < $(LOG_FILE)) lines"; \
	else \
		echo "✗ Debug log NOT created!"; \
	fi

# Show character mapping statistics from the log
.PHONY: analyze
analyze:
	@if [ ! -f $(LOG_FILE) ]; then \
		echo "Error: Log file not found. Run 'make test' first."; \
		exit 1; \
	fi
	@echo "==== CHARACTER MAPPING ANALYSIS ===="
	@echo ""
	@echo "Total render requests:"
	@grep "RENDER char=" $(LOG_FILE) | wc -l
	@echo ""
	@echo "Characters found in unicode_map:"
	@grep "method=unicode_map" $(LOG_FILE) | wc -l
	@echo ""
	@echo "Characters found via direct_index:"
	@grep "method=direct_index" $(LOG_FILE) | wc -l
	@echo ""
	@echo "Characters found via cp437_map:"
	@grep "method=cp437_map" $(LOG_FILE) | wc -l
	@echo ""
	@echo "Characters NOT_FOUND:"
	@grep "result=NOT_FOUND" $(LOG_FILE) | wc -l
	@echo ""
	@echo "==== UNIQUE CHARACTERS RENDERED ===="
	@grep "RENDER char=" $(LOG_FILE) | sed "s/.*char='\(.\)'.*/\1/" | sort -u | head -20
	@echo ""
	@echo "==== SAMPLE BOX DRAWING CHARACTERS ===="
	@grep "RENDER char=" $(LOG_FILE) | grep -E "char='[─│┌┐└┘═║╔╗╚╝]'" | head -10
	@echo ""
	@echo "==== NOT FOUND CHARACTERS ===="
	@grep "result=NOT_FOUND" $(LOG_FILE) | head -10

# Show the first 50 lines of the log
.PHONY: log-head
log-head:
	@if [ -f $(LOG_FILE) ]; then \
		head -50 $(LOG_FILE); \
	else \
		echo "Error: Log file not found. Run 'make test' first."; \
	fi

# Show the last 50 lines of the log
.PHONY: log-tail
log-tail:
	@if [ -f $(LOG_FILE) ]; then \
		tail -50 $(LOG_FILE); \
	else \
		echo "Error: Log file not found. Run 'make test' first."; \
	fi

# View the full log
.PHONY: log
log:
	@if [ -f $(LOG_FILE) ]; then \
		less $(LOG_FILE); \
	else \
		echo "Error: Log file not found. Run 'make test' first."; \
	fi

# Clean generated files
.PHONY: clean
clean:
	@echo "Cleaning test outputs..."
	@rm -f $(OUTPUT_GIF) $(LOG_FILE)
	@echo "Clean complete."

# Clean everything including build artifacts
.PHONY: distclean
distclean: clean
	@echo "Cleaning build artifacts..."
	@cargo clean
	@echo "Distclean complete."

# Run all: build, test, and analyze
.PHONY: all
all: test analyze

# Show help
.PHONY: help
help:
	@echo "Font Rendering Debug Test Makefile"
	@echo "=================================="
	@echo ""
	@echo "Targets:"
	@echo "  make build      - Build the ttyvid binary"
	@echo "  make test       - Run font rendering test with debug logging"
	@echo "  make analyze    - Analyze character mapping from debug log"
	@echo "  make log-head   - Show first 50 lines of debug log"
	@echo "  make log-tail   - Show last 50 lines of debug log"
	@echo "  make log        - View full debug log in less"
	@echo "  make clean      - Remove generated test files"
	@echo "  make distclean  - Remove all generated files including build artifacts"
	@echo "  make all        - Run build, test, and analyze"
	@echo "  make help       - Show this help message"
	@echo ""
	@echo "Configuration:"
	@echo "  Input:  $(CAST_INPUT)"
	@echo "  Font:   $(FONT_FILE)"
	@echo "  Output: $(OUTPUT_GIF)"
	@echo "  Log:    $(LOG_FILE)"
	@echo "  FPS:    $(FPS)"
	@echo "  Theme:  $(THEME)"

.DEFAULT_GOAL := help

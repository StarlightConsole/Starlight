### Dev Configuration ###

BSP ?= rpi3
DEV_SERIAL ?= /dev/tty.usbserial-0001
STARSHIP_PATH ?= /Users/yolocat/Projects/starlight/starship
DEBUG_PRINTS ?= 0


### End of Configuration ###

define color_header
	@tput setaf 6 2> /dev/null || true
	@printf '\n%s\n' $(1)
	@tput sgr0 2> /dev/null || true
endef

define color_progress_prefix
	@tput setaf 2 2> /dev/null || true
	@tput bold 2 2> /dev/null || true
	@printf '%12s ' $(1)
	@tput sgr0 2> /dev/null || true
endef

ifeq ($(shell uname -s),Linux)
	DU_ARGUMENTS = --block-size=1024 --apparent-size
else ifeq ($(shell uname -s),Darwin)
	DU_ARGUMENTS = -k -A
endif

define disk_usage_KiB
	@printf '%s KiB\n' `du $(DU_ARGUMENTS) $(1) | cut -f1`
endef

QEMU_MISSING_STRING = "This board is not yet supported for QEMU."

ifeq ($(BSP),rpi3)
	TARGET = aarch64-unknown-none-softfloat
	KERNEL_BIN = kernel8.img
	QEMU_BINARY = qemu-system-aarch64
	QEMU_MACHINE_TYPE = raspi3b
	QEMU_RELEASE_ARGS = -display none -serial stdio
	OBJDUMP_BINARY = aarch64-elf-objdump
	NM_BINARY = aarch64-elf-nm
	READELF_BINARY = aarch64-elf-readelf
	GDB_BINARY = aarch64-elf-gdb
	LD_SCRIPT_PATH = $(shell pwd)/src/bsp/rpi
	RUSTC_MISC_ARGS = -C target-cpu=cortex-a53
else ifeq ($(BSP),rpi4)
	TARGET = aarch64-unknown-none-softfloat
	KERNEL_BIN = kernel8.img
	QEMU_BINARY = qemu-system-aarch64
	QEMU_MACHINE_TYPE = raspi4b
	QEMU_RELEASE_ARGS = -display none -serial stdio
	OBJDUMP_BINARY = aarch64-elf-objdump
	NM_BINARY = aarch64-elf-nm
	READELF_BINARY = aarch64-elf-readelf
	GDB_BINARY = aarch64-elf-gdb
	LD_SCRIPT_PATH = $(shell pwd)/src/bsp/rpi
	RUSTC_MISC_ARGS = -C target-cpu=cortex-a72
endif

export LD_SCRIPT_PATH

KERNEL_MANIFEST = Cargo.toml
KERNEL_LINKER_SCRIPT = kernel.ld
LAST_BUILD_CONFIG = target/$(BSP)_$(DEBUG_PRINTS).build_config
KERNEL_ELF_RAW = target/$(TARGET)/debug/kernel
KERNEL_ELF_RAW_DEPS = $(filter-out %: ,$(file < $(KERNEL_ELF).d)) $(KERNEL_MANIFEST) $(LAST_BUILD_CONFIG)

TT_TOOL_PATH = tools/translation_table_tool
KERNEL_ELF_TTABLES = target/$(TARGET)/debug/kernel+ttables
KERNEL_ELF = $(KERNEL_ELF_TTABLES)
KERNEL_ELF_SYMBOLS = target/$(TARGET)/debug/kernel.sym

FEATURES = --features bsp_$(BSP)
ifeq ($(DEBUG_PRINTS),1)
	FEATURES += --features debug_prints
endif

RUSTFLAGS = $(RUSTC_MISC_ARGS) -C link-arg=--library-path=$(LD_SCRIPT_PATH) -C link-arg=--script=$(KERNEL_LINKER_SCRIPT)
RUSTFLAGS_PEDANTIC = $(RUSTFLAGS) -D warnings -D missing_docs
COMPILER_ARGS = --target=$(TARGET) $(FEATURES)

RUSTC_CMD = cargo rustc $(COMPILER_ARGS) -Z build-std=core,alloc --manifest-path $(KERNEL_MANIFEST)
DOC_CMD = cargo doc $(COMPILER_ARGS)
CLIPPY_CMD = cargo clippy $(COMPILER_ARGS)
OBJCOPY_CMD = rust-objcopy

COMET_DEBUG_CMD = comet debug
COMET_DEBUG_ARGS = --port $(DEV_SERIAL)

COMET_UPLOAD_CMD = comet upload
COMET_UPLOAD_ARGS = --port $(DEV_SERIAL) --file kernel8.img

COMET_TEST_CMD = comet test
COMET_TEST_ARGS = --qemu-bin $(QEMU_BINARY) --upload-file $(KERNEL_BIN)

EXEC_QEMU = $(QEMU_BINARY) -M $(QEMU_MACHINE_TYPE)
EXEC_TT_TOOL = ruby $(TT_TOOL_PATH)/main.rb

.PHONY: all doc qemu qemu-asm qemu-debug gdb debug upload test clippy clean readelf objdump nm check

all: $(KERNEL_BIN)

$(LAST_BUILD_CONFIG):
	@rm -f target/*.build_config
	@mkdir -p target
	@touch $(LAST_BUILD_CONFIG)

$(KERNEL_ELF_RAW): $(KERNEL_ELF_RAW_DEPS) .FORCE
	$(call color_header, "Compiling kernel ELF - $(BSP)")
	@RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" $(RUSTC_CMD)

.FORCE:

$(KERNEL_ELF_TTABLES): $(KERNEL_ELF_RAW)
	$(call color_header, "Precomputing kernel translation tables")
	@cp $(KERNEL_ELF_RAW) $(KERNEL_ELF_TTABLES)
	@$(EXEC_TT_TOOL) $(BSP) $(KERNEL_ELF_TTABLES)

$(KERNEL_ELF_SYMBOLS): $(KERNEL_ELF_TTABLES)
	$(call color_header, "Generating kernel symbols")
	@$(OBJCOPY_CMD) --only-keep-debug -O elf64-aarch64 $(KERNEL_ELF_TTABLES) $(KERNEL_ELF_SYMBOLS)

$(KERNEL_BIN): $(KERNEL_ELF_TTABLES) $(KERNEL_ELF_SYMBOLS)
	$(call color_header, "Generating stripped binary")
	@$(OBJCOPY_CMD) --strip-all -O binary $(KERNEL_ELF_TTABLES) $(KERNEL_BIN)
	$(call color_progress_prefix, "Name")
	@echo $(KERNEL_BIN)
	$(call color_progress_prefix, "Size")
	$(call disk_usage_KiB, $(KERNEL_BIN))

doc:
	$(call color_header, "Generating docs")
	@$(DOC_CMD) --document-private-items --open

ifeq ($(QEMU_MACHINE_TYPE),)

qemu qemu-asm qemu-debug:
	$(call color_header, "$(QEMU_MISSING_STRING)")

else

qemu: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU")
	@$(EXEC_QEMU) $(QEMU_RELEASE_ARGS) -kernel $(KERNEL_BIN)

qemu-asm: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU in assembly mode")
	@$(EXEC_QEMU) $(QEMU_RELEASE_ARGS) -kernel $(KERNEL_BIN) -d in_asm

qemu-debug: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU in debug mode")
	@$(EXEC_QEMU) $(QEMU_RELEASE_ARGS) -kernel $(KERNEL_BIN) -s -S

endif

gdb: $(KERNEL_ELF_SYMBOLS)
	@$(GDB_BINARY) --quiet --tui -ex "target remote :1234" -ex "load" $(KERNEL_ELF_SYMBOLS)

debug:
	@$(COMET_DEBUG_CMD) $(COMET_DEBUG_ARGS)

upload: $(KERNEL_BIN)
	@$(COMET_UPLOAD_CMD) $(COMET_UPLOAD_ARGS)

test: $(KERNEL_BIN)
	@$(MAKE) -C $(STARSHIP_PATH)
	@$(COMET_TEST_CMD) $(COMET_TEST_ARGS) --qemu-args "-M $(QEMU_MACHINE_TYPE) $(QEMU_RELEASE_ARGS) -kernel $(STARSHIP_PATH)/kernel8.img"

test-asm: $(KERNEL_BIN)
	@$(MAKE) -C $(STARSHIP_PATH)
	@$(COMET_TEST_CMD) $(COMET_TEST_ARGS) --qemu-args "-M $(QEMU_MACHINE_TYPE) $(QEMU_RELEASE_ARGS) -kernel $(STARSHIP_PATH)/kernel8.img -d in_asm"

clippy:
	@RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" $(CLIPPY_CMD)

clean:
	rm -rf target $(KERNEL_BIN)

readelf: $(KERNEL_ELF)
	$(call color_header, "Reading ELF file")
	@$(READELF_BINARY) --headers $(KERNEL_ELF)

objdump: $(KERNEL_ELF)
	$(call color_header, "Disassembling ELF file")
	@$(OBJDUMP_BINARY) --disassemble --demangle --section .text --section .rodata $(KERNEL_ELF) | rustfilt

nm: $(KERNEL_ELF_SYMBOLS)
	$(call color_header, "Reading symbols")
	@$(NM_BINARY) --demangle --print-size $(KERNEL_ELF_SYMBOLS) | sort | rustfilt

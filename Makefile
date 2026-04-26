# Compile the RAL DSL examples into C headers and Rust modules.
# Each generation job is independent — pass `-j` for full parallelism.

RAL := ./target/release/ral
RIS := examples/riscv

# Rebuild the ral compiler if any of its sources change.
RAL_SRCS := $(shell find src -name '*.rs') Cargo.toml Cargo.lock

# Registers whose layout is identical at every width — single source, both archs.
SIMPLE_REGS := mtvec mcause mie mip

# satp needs arch-specific config values.
SATP_X64_FLAGS := -D xlen=64 -D mode_bits=4 -D asid_bits=16
SATP_X32_FLAGS := -D xlen=32 -D mode_bits=1 -D asid_bits=9

ALL_REGS := $(SIMPLE_REGS) satp mstatus

OUTPUTS := \
	$(foreach r,$(ALL_REGS),$(RIS)/x64/$(r).c $(RIS)/x64/$(r).rs) \
	$(foreach r,$(ALL_REGS),$(RIS)/x32/$(r).c $(RIS)/x32/$(r).rs)

.PHONY: all examples clean
.DELETE_ON_ERROR:

all: examples
examples: $(OUTPUTS)

$(RAL): $(RAL_SRCS)
	cargo build --release

# Ensure output dirs exist (order-only — won't trigger rebuilds on dir mtime).
$(OUTPUTS): | $(RIS)/x64 $(RIS)/x32

$(RIS)/x64 $(RIS)/x32:
	mkdir -p $@

# --- xlen-only parameterized registers (mtvec, mcause, mie, mip) ---
$(RIS)/x64/%.c: $(RIS)/%.ral $(RAL)
	$(RAL) $< c -D xlen=64 > $@
$(RIS)/x64/%.rs: $(RIS)/%.ral $(RAL)
	$(RAL) $< rust -D xlen=64 > $@
$(RIS)/x32/%.c: $(RIS)/%.ral $(RAL)
	$(RAL) $< c -D xlen=32 > $@
$(RIS)/x32/%.rs: $(RIS)/%.ral $(RAL)
	$(RAL) $< rust -D xlen=32 > $@

# --- satp: needs more config vars than just xlen ---
$(RIS)/x64/satp.c: $(RIS)/satp.ral $(RAL)
	$(RAL) $< c $(SATP_X64_FLAGS) > $@
$(RIS)/x64/satp.rs: $(RIS)/satp.ral $(RAL)
	$(RAL) $< rust $(SATP_X64_FLAGS) > $@
$(RIS)/x32/satp.c: $(RIS)/satp.ral $(RAL)
	$(RAL) $< c $(SATP_X32_FLAGS) > $@
$(RIS)/x32/satp.rs: $(RIS)/satp.ral $(RAL)
	$(RAL) $< rust $(SATP_X32_FLAGS) > $@

# --- mstatus: separate source per arch (field set differs structurally) ---
$(RIS)/x64/mstatus.c: $(RIS)/mstatus.ral $(RAL)
	$(RAL) $< c > $@
$(RIS)/x64/mstatus.rs: $(RIS)/mstatus.ral $(RAL)
	$(RAL) $< rust > $@
$(RIS)/x32/mstatus.c: $(RIS)/mstatus_rv32.ral $(RAL)
	$(RAL) $< c > $@
$(RIS)/x32/mstatus.rs: $(RIS)/mstatus_rv32.ral $(RAL)
	$(RAL) $< rust > $@

clean:
	rm -f $(OUTPUTS)

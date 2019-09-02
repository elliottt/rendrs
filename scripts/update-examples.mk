
EXAMPLES := $(wildcard examples/*.png)

.PHONY: all
all: $(EXAMPLES)

examples/%.png: scenes/%.yaml
	cargo run --release -- $< $@

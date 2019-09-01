
EXAMPLES := $(wildcard examples/*.png)

all: $(EXAMPLES)

examples/%.png: scenes/%.yaml
	cargo run --release -- $< $@

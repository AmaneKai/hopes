CC      ?= cc
PREFIX  ?= /usr/local
BUILD   := build
BIN     := $(BUILD)/hopes
UNAME   := $(shell uname -s)

CFLAGS  ?= -Oz -flto
CFLAGS  += -std=gnu11 -Wall -Wextra -Wno-unused-function -Isrc \
           -fno-asynchronous-unwind-tables -fno-unwind-tables -fno-stack-protector -fno-ident \
           -ffunction-sections -fdata-sections

ifneq (,$(findstring MINGW,$(UNAME))$(findstring MSYS,$(UNAME)))
BIN      = $(BUILD)/hopes.exe
CFLAGS  += -DPDC_WIDE
LDFLAGS += -static -Wl,--gc-sections
LIBS     = -lpdcurses
else ifeq ($(UNAME),Darwin)
LDFLAGS += -flto -Wl,-dead_strip -Wl,-x -Wl,-no_fixup_chains -Wl,-no_data_const
LIBS     = -lncurses
else
LDFLAGS += -no-pie -Wl,--gc-sections -Wl,--as-needed -Wl,-z,norelro -Wl,-z,noseparate-code -Wl,--build-id=none
LIBS     = $(shell pkg-config --libs ncursesw 2>/dev/null || echo -lncursesw)
CFLAGS  += $(shell pkg-config --cflags ncursesw 2>/dev/null)
endif

ifdef SQLITE
CFLAGS += -DHOPES_SQLITE
LIBS   += -lsqlite3
endif

HEADERS := $(wildcard src/*.h)
SOURCES := $(filter-out src/amane.h,$(HEADERS)) src/main.c tests/tests.c

ifeq ($(UNAME),Darwin)
SIZE_BUDGET ?= 72000
else
SIZE_BUDGET ?= 63000
endif

all: $(BIN)

$(BIN): src/main.c $(HEADERS) | $(BUILD)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ src/main.c $(LIBS)
	strip $(if $(filter Darwin,$(UNAME)),-x,-s) $@

$(BUILD):
	mkdir -p $@

test: $(BUILD)/tests
	$(BUILD)/tests

$(BUILD)/tests: tests/tests.c $(HEADERS) | $(BUILD)
	$(CC) $(CFLAGS) -O0 -g -fsanitize=address,undefined -DHOPES_NO_UI -o $@ tests/tests.c $(if $(SQLITE),-lsqlite3)

compile_commands.json: Makefile
	@printf '[\n  {"directory": "%s", "file": "src/main.c", "arguments": ["%s", "-std=gnu11", "-Isrc", "-c", "src/main.c"]},\n  {"directory": "%s", "file": "tests/tests.c", "arguments": ["%s", "-std=gnu11", "-Isrc", "-DHOPES_NO_UI", "-c", "tests/tests.c"]}\n]\n' \
		"$(CURDIR)" "$(CC)" "$(CURDIR)" "$(CC)" > $@

install: $(BIN)
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/hopes

clean:
	rm -rf $(BUILD)

$(BUILD)/lint: scripts/lint.c | $(BUILD)
	$(CC) -std=gnu11 -Wall -Wextra -O1 -o $@ scripts/lint.c

lint: $(BUILD)/lint
	$(BUILD)/lint comments $(SOURCES)
	$(BUILD)/lint nesting $(SOURCES)

strict:
	$(CC) -std=gnu11 -Wall -Wextra -Werror -Wno-unused-function -Isrc $(CFLAGS_STRICT) -fsyntax-only src/main.c
	$(CC) -std=gnu11 -Wall -Wextra -Werror -Wno-unused-function -Isrc -DHOPES_NO_UI -fsyntax-only tests/tests.c

size: $(BIN)
	sh scripts/check-size.sh $(BIN) $(SIZE_BUDGET)

sync-test: $(BIN)
	sh tests/sync.sh $(BIN)

check: lint strict test size sync-test

.PHONY: all test lint strict size sync-test check install clean

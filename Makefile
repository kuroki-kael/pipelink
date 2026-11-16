PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin
DOCDIR ?= $(PREFIX)/share/doc/pipelink
LICENSEDIR ?= $(PREFIX)/share/licenses/pipelink
SYSTEMD_USER_DIR ?= $(PREFIX)/lib/systemd/user
CARGO ?= cargo

.PHONY: all build install uninstall

all: build

build:
	$(CARGO) build --release -p pipelink --locked

install:
	install -Dm755 target/release/pipelink "$(DESTDIR)$(BINDIR)/pipelink"
	install -d "$(DESTDIR)$(SYSTEMD_USER_DIR)"
	sed "s|ExecStart=/usr/bin/pipelink|ExecStart=$(BINDIR)/pipelink|g" \
		packaging/systemd/pipelink.service > "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	chmod 644 "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	install -Dm644 packaging/config.example.json "$(DESTDIR)$(DOCDIR)/config.example.json"
	install -Dm644 LICENSE "$(DESTDIR)$(LICENSEDIR)/LICENSE"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/pipelink"
	rm -f "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	rm -f "$(DESTDIR)$(DOCDIR)/config.example.json"
	rm -f "$(DESTDIR)$(LICENSEDIR)/LICENSE"
	rmdir --ignore-fail-on-non-empty "$(DESTDIR)$(DOCDIR)" "$(DESTDIR)$(LICENSEDIR)" 2>/dev/null || true

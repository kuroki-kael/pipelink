PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin
DOCDIR ?= $(PREFIX)/share/doc/pipelink
LICENSEDIR ?= $(PREFIX)/share/licenses/pipelink
SYSTEMD_USER_DIR ?= $(PREFIX)/lib/systemd/user
APPLICATIONS_DIR ?= $(PREFIX)/share/applications
ICON_THEME_DIR ?= $(PREFIX)/share/icons/hicolor
CARGO ?= cargo
CARGO_FEATURES ?=
PIPELINK_BIN ?= target/release/pipelink
ICON_SIZES := 16x16 32x32 48x48 64x64 128x128 256x256 512x512

.PHONY: all build install install-gui uninstall

all: build

build:
	$(CARGO) build --release -p pipelink --locked $(if $(CARGO_FEATURES),--features $(CARGO_FEATURES),)

install:
	install -Dm755 "$(PIPELINK_BIN)" "$(DESTDIR)$(BINDIR)/pipelink"
	install -d "$(DESTDIR)$(SYSTEMD_USER_DIR)"
	sed "s|ExecStart=/usr/bin/pipelink|ExecStart=$(BINDIR)/pipelink|g" \
		packaging/systemd/pipelink.service > "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	chmod 644 "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	install -Dm644 packaging/config.example.json "$(DESTDIR)$(DOCDIR)/config.example.json"
	install -Dm644 LICENSE "$(DESTDIR)$(LICENSEDIR)/LICENSE"

install-gui: install
	install -Dm644 packaging/desktop/pipelink.desktop "$(DESTDIR)$(APPLICATIONS_DIR)/pipelink.desktop"
	@for size in $(ICON_SIZES); do \
		install -Dm644 "assets/icons/hicolor/$${size}/apps/pipelink.png" \
			"$(DESTDIR)$(ICON_THEME_DIR)/$${size}/apps/pipelink.png"; \
	done
	install -Dm644 assets/icons/hicolor/scalable/apps/pipelink.svg \
		"$(DESTDIR)$(ICON_THEME_DIR)/scalable/apps/pipelink.svg"
	install -Dm644 assets/icons/hicolor/symbolic/apps/pipelink-symbolic.svg \
		"$(DESTDIR)$(ICON_THEME_DIR)/symbolic/apps/pipelink-symbolic.svg"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/pipelink"
	rm -f "$(DESTDIR)$(SYSTEMD_USER_DIR)/pipelink.service"
	rm -f "$(DESTDIR)$(DOCDIR)/config.example.json"
	rm -f "$(DESTDIR)$(LICENSEDIR)/LICENSE"
	rm -f "$(DESTDIR)$(APPLICATIONS_DIR)/pipelink.desktop"
	@for size in $(ICON_SIZES); do \
		rm -f "$(DESTDIR)$(ICON_THEME_DIR)/$${size}/apps/pipelink.png"; \
	done
	rm -f "$(DESTDIR)$(ICON_THEME_DIR)/scalable/apps/pipelink.svg"
	rm -f "$(DESTDIR)$(ICON_THEME_DIR)/symbolic/apps/pipelink-symbolic.svg"
	rmdir --ignore-fail-on-non-empty "$(DESTDIR)$(DOCDIR)" "$(DESTDIR)$(LICENSEDIR)" 2>/dev/null || true

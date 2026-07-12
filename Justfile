signing_identity := env_var_or_default("HANDY_SIGNING_IDENTITY", "Handy Dev")
certificate_sha1 := env_var_or_default("HANDY_CERTIFICATE_SHA1", "E3262DC4B5253DFA15864022B77D68B35CBA8302")
app_source := "src-tauri/target/release/bundle/macos/Handy.app"
app_destination := env_var_or_default("HANDY_APP_PATH", "/Applications/Handy.app")
helper_label := "com.falleng0d.karabiner-input.helper.local"
installed_helper := "/Library/PrivilegedHelperTools/com.falleng0d.karabiner-input.helper"

export KARABINER_INPUT_LOCAL_CERT_SHA1 := certificate_sha1
export APPLE_SIGNING_IDENTITY := signing_identity

default:
	@just --list

# Build and sign the local release app.
local-build: local-check-certificate
	bunx tauri build --bundles app \
		--config '{"bundle":{"createUpdaterArtifacts":false,"macOS":{"signingIdentity":"{{ signing_identity }}"}}}'

# Backward-compatible alias for the original recipe.
release: local-build

# Confirm the configured certificate and private key are available.
local-check-certificate:
	@actual="$(security find-certificate -c "{{ signing_identity }}" -Z | awk '/SHA-1/{print $3; exit}')"; \
		test "$actual" = "{{ certificate_sha1 }}" || { \
			echo "Expected {{ signing_identity }} certificate {{ certificate_sha1 }}, found ${actual:-none}" >&2; \
			exit 1; \
		}
	@security find-identity -v -p codesigning | rg -F '"{{ signing_identity }}"' >/dev/null

# Stop the installed app if it is running.
local-stop:
	@osascript -e 'tell application id "com.pais.handy" to quit' 2>/dev/null || true; \
		for _ in {1..50}; do \
			pgrep -f '^{{ app_destination }}/Contents/MacOS/handy$' >/dev/null || exit 0; \
			sleep 0.1; \
		done; \
		echo "Handy did not exit within 5 seconds" >&2; \
		exit 1

# Copy the latest signed release into /Applications.
local-install-app: local-stop
	test -d "{{ app_source }}"
	ditto "{{ app_source }}" "{{ app_destination }}"
	codesign --verify --deep --strict --verbose=2 "{{ app_destination }}"

# Install or replace the local root helper. Prompts for administrator approval.
local-install-helper:
	bash scripts/install-karabiner-helper-local.sh "{{ app_destination }}"

# Verify app/helper signatures, service state, plist, and IPC socket.
local-verify:
	codesign --verify --deep --strict --verbose=2 "{{ app_destination }}"
	codesign -R='identifier "com.pais.handy" and certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ app_destination }}/Contents/MacOS/handy"
	codesign -R='certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ app_destination }}/Contents/MacOS/karabiner-input-helper"
	codesign -R='certificate leaf = H"{{ certificate_sha1 }}"' --verify --verbose=2 "{{ installed_helper }}"
	plutil -lint "{{ app_destination }}/Contents/Library/LaunchDaemons/com.falleng0d.karabiner-input.helper.plist"
	launchctl print "system/{{ helper_label }}" | rg 'state = running'
	test -S /var/run/karabiner-input.sock
	test "$(stat -f '%Su:%Sp' /var/run/karabiner-input.sock)" = "${USER}:srw-------"

# Show the latest Handy application log entries.
local-logs:
	tail -n 200 "${HOME}/Library/Logs/com.pais.handy/handy.log"

# Follow Handy application logs while reproducing a problem.
local-logs-follow:
	tail -f "${HOME}/Library/Logs/com.pais.handy/handy.log"

# Show recent privileged helper logs.
local-helper-logs:
	tail -n 200 /var/log/karabiner-input-helper.log

# Follow privileged helper logs while reproducing a problem.
local-helper-logs-follow:
	tail -f /var/log/karabiner-input-helper.log

# Print service, process, socket, signature, and recent error diagnostics.
local-diagnose:
	@echo '=== Launch daemon ==='
	@launchctl print "system/{{ helper_label }}" || true
	@echo '=== Processes ==='
	@pgrep -alf 'Handy.app/Contents/MacOS/handy|karabiner-input-helper' || true
	@echo '=== IPC socket ==='
	@ls -la /var/run/karabiner-input.sock || true
	@echo '=== Installed signatures ==='
	@codesign --verify --deep --strict --verbose=2 "{{ app_destination }}" || true
	@codesign --verify --strict --verbose=2 "{{ installed_helper }}" || true
	@echo '=== Recent Handy errors ==='
	@rg -i 'Failed to paste|Using paste method: Karabiner|Karabiner input helper|helper rejected|unsupported input' "${HOME}/Library/Logs/com.pais.handy/handy.log" | tail -n 100 || true
	@echo '=== Recent helper logs ==='
	@tail -n 100 /var/log/karabiner-input-helper.log 2>/dev/null || echo 'No helper log yet; rerun just local-install-helper to enable it.'

# Relaunch the installed app.
local-restart: local-stop
	open "{{ app_destination }}"

# Build, install, verify, and restart everything needed for local iteration.
local-all: local-build
	just local-install-app
	just local-install-helper
	just local-verify
	just local-restart

# Remove only the local-development launch daemon and helper.
local-uninstall-helper:
	sudo launchctl bootout "system/{{ helper_label }}" 2>/dev/null || true
	sudo rm -f "/Library/LaunchDaemons/{{ helper_label }}.plist" "{{ installed_helper }}" /var/run/karabiner-input.sock /var/log/karabiner-input-helper.log

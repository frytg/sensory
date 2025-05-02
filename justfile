set export := true

default:
	just --list

# use a default sops file, or allow to be overridden by SOPS_ENV_FILE environment variable
DEFAULT_SOPS_FILE:= '.env.sops.yaml'
SELECTED_SOPS_FILE:= env('SOPS_ENV_FILE', DEFAULT_SOPS_FILE)

# run a command with the selected sops file (injecting environment variables)
env *args:
	sops exec-env --same-process {{SELECTED_SOPS_FILE}} "{{args}}"

## ---------------------------------

release binary:
	just --yes decrypt-key sensor-config.sops.json
	just env "cargo build --release --bin {{binary}}"

list-usb:
	ls /dev/tty.*

run binary:
	just --yes decrypt-key sensor-config.sops.json
	just env "cargo run --bin {{binary}} -- --port /dev/tty.usbmodem101"

flash binary:
	just release {{binary}}
	espflash flash --monitor --port /dev/tty.usbmodem101 --chip esp32c6 target/riscv32imac-unknown-none-elf/release/{{binary}}

logs:
	espflash monitor

format:
	cargo fmt

info:
	espflash board-info --port /dev/tty.usbmodem101

targets:
	rustc --print target-list


## ---------------------------------
## ENCRYPTION shortcuts

# add/ remove keys (if .sops.yaml setup was changed)
[group('ENCRYPTION')]
update-keys:
	just _update-key .env.sops.yaml

_update-key file:
	sops updatekeys {{file}}

# rotate keys (refreshed internal encryption keys)
[group('ENCRYPTION')]
rotate-keys:
	just _rotate-key .env.sops.yaml

_rotate-key file:
	sops rotate --in-place {{file}}

# list PGP keys and their fingerprints
[group('ENCRYPTION')]
list-pgp:
	gpg --list-keys

# make changes to a secret file
[group('ENCRYPTION')]
edit-key file:
	EDITOR=nano sops edit {{file}}

# decrypt a secret file
[group('ENCRYPTION')]
[confirm('This will overwrite any previously decrypted files, are you sure? (type `yes` to continue)')]
decrypt-key file:
	sops --output $(echo {{file}} | sed 's/\.sops//g') --decrypt {{file}}

# decrypt all secret files
[group('ENCRYPTION')]
[confirm('This will overwrite all previously decrypted files, are you sure? (type `yes` to continue)')]
decrypt:
	just decrypt-key .env.sops.yaml

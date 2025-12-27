# Platformer

Platformer is a fast-paced, self-hosted multiplayer action game.
Players compete in skill-based PvP matches using movement, attacks,
and abilities.

Games are hosted locally, with customizable teams, player settings, and
camera options via a simple configuration file. The focus is on
responsive controls and straightforward, competitive gameplay.

---

## Controls

All actions are directional, based on your aim, and use the standard
WASD layout:

- **W**: Aim up
- **A**: Move and aim left
- **S**: Fast fall and aim down
    Slam attack if landing on an enemy.
- **D**: Move and aim right
- **H**: Dash
    Lunge in aimed direction; can knock opponents away.
- **J**: Normal attack
    Knocks enemy in the aimed direction.
- **K**: Finisher / Stun
    Launches an enemy if they are in a combo. The launch velocity
    depends on the combo count.
- **LeftShift** or **L**: Parry
    Can only be done when standing on a platform.
    If an enemy attacks while you are parrying, they get stunned for
    the duration their attack would have stunned you.

## Configuration

Everything from player name to the number of players per team is
configured in config.toml. The file is fully customizable and includes
default values.

## Installation & Updates

The manager.sh script simplifies installing and updating of the client.
It ensures your configuration and assets are preserved while keeping
the binary up-to-date.

### Prerequisites

- Rust and Cargo (cargo build required)
- Git installed and repository cloned
- POSIX-compatible shell (`/bin/sh`)

### Installation

This step only needs to be done once per system.

```sh
./manager.sh install
```

### Updating

```sh
./manager.sh update
```

### Notes

Assets and configuration files are only copied if they don’t already
exist, ensuring user edits are preserved.

If ~/.local/bin is not in your $PATH, add it with:
```sh
export PATH="$HOME/.local/bin:$PATH"
```

Conflicts during stash pop or merge require manual resolution in
`config.toml`.

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

| Key      | Action                 | Notes                                                                                                                                                            |
| -------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W        | Aim up                 |                                                                                                                                                                  |
| A        | Move and aim left      |                                                                                                                                                                  |
| S        | Fast fall and aim down | Slam attack if landing on an enemy.                                                                                                                              |
| D        | Move and aim right     |                                                                                                                                                                  |
| H        | Dash                   | Can knock opponents away.                                                                                                                                        |
| J        | Normal attack          | Knocks enemy in the aimed direction.                                                                                                                             |
| K        | Finisher/Stun          | Launches an enemy if they are in a combo. The launch velocity depends on the combo count.                                                                        |
| L/LShift | Parry                  | Can only be done when standing on a platform. If an enemy attacks while you are parrying, they get stunned for the duration their attack would have stunned you. |

## Configuration

Everything from player name to the number of players per team is
configured in `config.toml`. The file is fully customizable and includes
default values.
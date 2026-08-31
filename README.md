# TurboRipent
![alt text](https://github.com/Outerbeast/TurboRipent/blob/main/menu_preview.png?raw=true)
Turbocharged entity ripping
## Features
A standalone TUI application for extracting, importing, and editing BSP entity data. This is designed to be a direct replacement for the standard Ripent.exe provided by the GoldSrc compile tools via ZHLT/VHLT.
Includes a basic entity editor for quick edits.

- Extraction and importing entity (`.ent`) files
- Split extraction/import of point entities and brush entities (`.entp`/`.entm`)
- Entity repair — re-parses and re-serialises entity data, fixing corruption
- Editor mode (Windows only)
- BSP entity statistics (`.log`)

Importing also automatically fixes any corruption in the entity data and discards references to brush models that don't exist in the target BSP.

For a similar tool for much more powerful control over entity modifications, check out [Lazyripent](https://github.com/Zode/Lazyripent2).

## Installation
- Download the application from the [Releases](https://github.com/Outerbeast/TurboRipent/releases) section

That's it. You can launch the application by double clicking, or launch it from the terminal.
*<small>Note: On Linux, you can only launch the application from the terminal.<small>

## Usage

### Interactive Menu
Launching the application without arguments will display an interactive menu with the following options:
- **Extract** — Extract an entity list (`.ent`) from a BSP file
- **Import** — Import an entity list (`.ent`) into a BSP file
- **Split Extract** — Extract separate `.entp` (point entities) and `.entm` (brush entities)
- **Split Import** — Import `.entp`/`.entm` files into a BSP (both files required)
- **Repair** — Re-parse and re-serialise entity data, fixing corruption
- **Stats** — Display BSP entity statistics (can save as `.log`)
- **Editor** — Open the graphical entity editor for a BSP or ENT file
- **Help** — Show usage information
- **Exit** — Close TurboRipent

You can change an option by pressing the Up/Down keys and selecting via Enter/Spacebar.

After selecting an option you will be instructed to provide the necessary files and paths, which you can drag into the window or enter manually.

### Quick Extract/Import
Drag files onto the executable (or pass them as CLI arguments):

- **.bsp** - Extract entity data as a `.ent` file
- **.ent** - Import entity data into the corresponding `.bsp` (the `.ent` file is then deleted)
- **.entp / .entm** - Import split entity data into the corresponding `.bsp` (both files deleted after import)

Example:

`level.bsp` -> `level.ent` (extract)
`level.ent` -> Deleted after successful import, fails if `level.bsp` is missing.
`level.entp` OR `level.ent` -> Deleted after successful import, fails if `level.bsp` is missing or if one of the brush/point entity pair is missing

### Terminal Editor

![alt text](https://github.com/Outerbeast/TurboRipent/blob/main/editor_preview.png?raw=true)

The editor is a simple graphical interface for viewing and editing entities within a BSP or ENT file. <br>Please note: there is no FGD support. The editor is intended to be used to make quick edits and assumes you know what you're doing with regards to entity keys and the flags needing to be set. If you want a proper entity editor that includes FGD support, use [bspguy](https://github.com/wootguy/bspguy).


To launch the editor:
`TurboRipent.exe -edit <file>` (or `-editor`/`-gui`) to launch the terminal editor.
You may also drag a BSP or ENT file onto `TurboRipent-Editor.cmd`.

### Entity List
All entities are listed by classname on the left panel. Selecting one displays its key/value pairs and set Entity flags (`spawnflags`). Below the entity list is a search box that you can filter entities in the list that have a key or value matching the search query.

### Entity Properties
The Properties panel on the right shows the keyvalues for the selected entity.
Clicking "➕" will add a new blank keyvalue row which you can add your key and value you want

Clicking on "❌" will delete an existing keyvalue row.

### Entity Flags
The Flags panel shows a number of flag boxes that can be (un)checked by clicking on them. This corresponds to the `spawnflags` key up in the Entity Properties. To know which flags to set for your entity, refer to the FGD file or the entity docs.

### Buttons
- 🆕`Create` — Creates a new entity with classname `new_entity`
- 🖨️`Clone` — Duplicates the selected entity
- 🗑️`Delete` — Removes the selected entity
- ↩️`Undo` - Reverts a change done to an entity
- ↪️`Redo` - Restores a change done to an entity
- 💾`Save` — Saves changes and exits the editor

Note: Closing the editor via `X` in the title bar will exit **without saving any changes**, keep this in mind.

### Command Line Arguments

| Argument | Description |
|----------|-------------|
| *(none)* | Launch the interactive TUI menu |
| `-help` / `-usage` / `-h` | Show usage information and exit |
| `-stats` / `-info` `<file>` | Show BSP entity statistics |
| `-extract` / `-export` / `-e` `<file>` | Extract entity data from a `.bsp` or import from `.ent`/`.entp`/`.entm` based on file extension |
| `-import` / `-i` `<file>` | Import entity data into a `.bsp` from `.ent`/`.entp`/`.entm` |
| `-splitextract` / `-splitexport` / `-se` `<file>` | Extract split `.entp` (point entities) and `.entm` (brush entities) from a BSP |
| `-splitimport` / `-si` `<file>` | Import split `.entp`/`.entm` files into a BSP (both files required) |
| `-repair` / `-parse` / `-r` `<file>` | Re-parse and re-serialise entity data, fixing corruption |
| `-edit` / `-editor` / `-gui` `<file>` | Open the terminal entity editor |
| `<file1>` `<file2>` `...` | Quick mode — auto-detect action based on file extension (see [Quick Extract/Import](#quick-extractimport)) |


## Building from Source

### Prerequisites

- [Rust toolchain](https://rustup.rs/) installed

### Build Instructions
1. [Download](https://github.com/Outerbeast/TurboRipent/archive/refs/heads/main.zip) or clone the repository:

```cmd
git clone https://github.com/Outerbeast/TurboRipent.git
cd TurboRipent
```
2. Build using the script:
- Double-click `build.cmd` or run it manually:
```
build.cmd
```

Alternatively, build directly with Cargo:
```
cargo build --release
```

The executable will be generated in `target/release/TurboRipent.exe`.

## License
See [LICENSE](LICENSE) for details.

## Feedback & Issues
If you have feedback or encounter issues, please open an issue on [GitHub Issues](https://github.com/Outerbeast/TurboRipent/issues).

---

Thank you for using TurboRipent!

### Credits
- **Outerbeast** - Author
- **Garompa** - Testing and feedback

Terminal menu powered by:-
- crossterm
- dialoguer

Editor powered by [Cursive](https://github.com/gyscos/cursive) with the crossterm backend.

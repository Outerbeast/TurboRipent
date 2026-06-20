# TurboRipent
![alt text](https://github.com/Outerbeast/TurboRipent/blob/main/menu_preview.png?raw=true)
Turbocharged entity ripping
## Features
A standalone TUI application for extracting, importing, and editing BSP entity data files
Includes a basic entity editor for quick edits:

- Extraction and Importing entity (.ent) files
- Split extraction/import (.entp / .entm)
- Editor mode
- Print BSP entity statistics (.log)

## Installation
- Download the application from the [Releases](https://github.com/Outerbeast/TurboRipent/releases) section
- Run the executable

and you're done.

## Usage

### Interactive Menu
Launching the application without arguments will display an interactive menu with the following options:
- **Extract** — Extract an entity list (.ent) from a BSP file
- **Import** — Import an entity list (.ent) into a BSP file
- **Split Extract** — Extract separate .entp (point entities) and .entm (brush entities)
- **Split Import** — Import .entp/.entm files into a BSP (both files required)
- **Editor** — Open the graphical entity editor for a BSP or ENT file
- **Stats** — Display BSP entity statistics (can save as .log)
- **Help** — Show usage information

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

### Editor (Windows Only)

![alt text](https://github.com/Outerbeast/TurboRipent/blob/main/editor_preview.png?raw=true)

The editor is a simple graphical interface for viewing and editing entities within a BSP or ENT file.
To launch the editor:
`TurboRipent.exe -edit <file>` (or `-editor` / `-gui`) to launch the graphical editor.
You may also drag a BSP or ENT file onto `TurboRipent-Editor.cmd`.

All entities are listed by classname on the left. Selecting one displays its keyvalues on the right, formatted as:
```
key=value
```

A filter box below the list allows searching by key or value — matching entities update in real time.

Buttons:
- **Create** — Creates a new entity with classname `new_entity`
- **Clone** — Duplicates the selected entity
- **Delete** — Removes the selected entity
- **Save** — Saves changes and exits the editor

Closing the editor via `X` prompts you to confirm changes. Clicking `Save` will save changes and exit.

*<small>The editor is a work-in-progress which is why its very primitive and basic with not very many functions. The aim is to replace outdated applications like EntEd or BSPEdit, where entity data is simply displayed in plain text which makes entmapping difficult and error prone. Extensive feedback and testing is required.</small>

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

Editor powered by [Native Windows GUI](https://github.com/gabdube/native-windows-gui) - a big thanks to the NWG project for providing a Rust library to build native Windows applications.

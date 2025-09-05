# TurboRipent
Entity ripping at turbo speed
## Features
This is a TUI frontend for Ripent and [Lazyripent](https://github.com/Zode/Lazyripent2) which allow you to extract and import BSP entity data files quickly and easily
Includes a basic entity editor for quick edits:

- Extraction and Importing entity (.ent) files
- Applying Lazyripent rules (.rule) files
- Editor mode

Other Ripent available features such as:
- Texture import/export
- Print BSP statistics (.chart)
- Print BSP extents information (.ext)

To get up to speed how to create a rule file, visit [this page](https://github.com/Zode/Lazyripent2?tab=readme-ov-file#rule-file-syntax) to get details on rule file syntax.

## Installation
- Download the application from the [Releases](https://github.com/Outerbeast/TurboRipent/releases) section
- Run the exe for initial setup, this will search for your Sven Co-op game install.
Intial setup will save a config file (`TurboRipent_conf.json`) in `%LOCALAPPDATA%`

The config file is in this structure:

```JSON
{
  "RipentPath": "C:\\path\\to\\Ripent_x64.exe",
  "LazyripentPath": "C:\\path\\to\\lazyripent.exe",
  "Verbose": false,
}
```

If the application is finding trouble finding Ripent and Lazyripent, you can delete this config file and launch the application again to find their location. If the issue persists, try setting the paths manually, making sure to follow the format shown above.

## Usage
- You can launch the application directly where you will be presented with menu with choices

- Dragging BSP files onto the executable file will automatically extract the entity data from them as `.ent` files.

- Dragging ENT files onto the executable file will automatically import the entity data into the corresponding BSP files, if those BSP files exist in the same folder as the ENT files

- Dragging a BSP file onto `TurboRipent-Editor.cmd` will launch the BSP in the editor. You can also run the cmd file as is - TurboRipent will ask for a BSP to edit.

### Editor
With the built-in editor, it is possible to make quick entity edits to maps.
All of the entities in the BSP, represented by classnames, will be listed on the left in the entity list. These entities are selectable - when selecting one the entity's keyvalues are displayed in the box on the right and can be edited.
Entity keyvalues are formatted as such
```
key=value
```
Buttons:
- **Create**: creates a new entity with the classname `new_entity`
- **Clone**: makes a copy of the selected entity
- **Delete**: deletes the selected entity
- **Save** : saves changes and exits the editor

When closing the editor, via `X`, you will be prompted whether you want to confirm changes or not before exiting.

*<small>The editor is a work-in-progress which is why its very primitive and basic with not very many functions. The aim is to replace outdated applications like EntEd or BSPEdit, where entity data is simply displayed in plain text which makes entmapping difficult and error prone. Extensive feedback and testing is required.</small>

## Building from Source

### Prerequisites

- [Go toolchain](https://go.dev/dl/) installed (Go 1.25 or newer)
- [Ripent](steam://launch/276160) - Obtained from the Sven Co-op SDK from Steam
- [Lazyripent2](https://github.com/Zode/Lazyripent2) (not Lazyripent) - If this is missing the application will launch but applying `.rule`s and the editor will not function.

### Build Instructions
1. [Download](https://github.com/Outerbeast/TurboRipent/archive/refs/heads/main.zip) or clone the repository:

```cmd
git clone https://github.com/Outerbeast/TurboRipent.git
cd TurboRipent
```
2. Run the build script:
- Double-click `build.cmd` or run it manually:
```
build.cmd
```

The executable will be generated in the current directory.

## License
See [LICENSE](LICENSE) for details.

## Feedback & Issues
If you have feedback or encounter issues, please open an issue on [GitHub Issues](https://github.com/Outerbeast/TurboRipent/issues).

---

Thank you for using TurboRipent!

### Credits
- **Outerbeast** - Author
- **Garompa** - Testing and feedback

Special thanks to **Zode** for creating Lazyripent

/*
	TurboRipent - TUI Frontend for Ripent / Lazyripent
	Version 1.0

Copyright (C) 2025 Outerbeast
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
package main

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"runtime"
	"strings"
	"time"

	"golang.org/x/term"
)

var MENU [11]Option = [11]Option{

	{'0', "None", "", ""},
	{'1', "Extract", "-export", "\t\tExtract an entity list (.ent file) from a BSP file"},
	{'2', "Import", "-import", "\t\tImport an entity list (.ent file) into BSP file"},
	{'3', "Apply Rule", "-rule", "\t\tApplies a rulefile for lazyripent"},
	{'4', "Edit", "-edit", "\t\tOpens the entity editor"},
	{'5', "Texture Export", "-textureexport", "\tExport texture data from a BSP file"},
	{'6', "Texture Import", "-textureimport", "\tImport texture data to a BSP file"},
	{'7', "Write chart", "-chart", "\t\tWrites a .log file containing map statistics"},
	{'8', "Write extents", "-writeextentfile", "\tWrites a .ext file containing the map extents"},
	{'v', "Verbose output - OFF", "-verbose", "\t\tToggles verbose output"},
	{'h', "Help", "", "\t\tShow this help message"},
}

type Option struct {
	Input                       rune
	Name, Argument, Description string
}

func ClearTerminal() {

	var pCmd *exec.Cmd

	if runtime.GOOS == "windows" {
		pCmd = exec.Command("cmd", "/c", "cls")
	} else {
		pCmd = exec.Command("clear")
	}

	pCmd.Stdout = os.Stdout
	pCmd.Run()
}

func GetKeyStroke() rune { // Keystroke listener

	var last rune
	var lastTime time.Time
	buf := make([]byte, 1)

	for {

		os.Stdin.Read(buf)
		current := rune(buf[0])
		now := time.Now()
		// Ignore key being held down, wait for next keystroke
		if current != last || now.Sub(lastTime) > 500*time.Millisecond {

			last = current
			lastTime = now

			return current
		}
	}
}

func GetPromptInput(prompt string) string {

	fmt.Println(prompt)
	fmt.Print("\033[32m>\033[0m ")

	reader := bufio.NewReader(os.Stdin)
	line, _ := reader.ReadString('\n')
	line = strings.TrimSpace(line)

	// Remove surrounding quotes if present
	if len(line) >= 2 && line[0] == '"' && line[len(line)-1] == '"' {
		line = line[1 : len(line)-1]
	}

	return line
}

func ShowHelp() {

	ClearTerminal()
	fmt.Printf("\n%s\nThis tool allows you to extract and import BSP entity data.\nOptions:", ColouriseText("?-Help-?", BrightCyan, ""))

	for _, option := range MENU[1:11] {
		fmt.Printf("\t%s : %s\n", option.Name, option.Description)
	}

	fmt.Printf("\nThank you for using %s!\n", AppName)
}

func DisplayMenu() bool { // TUI menu - return true if menu should remain open, false if to close and quit the application
	// Save current terminal state
	oldState, err := term.MakeRaw(int(os.Stdin.Fd()))

	if err != nil {
		panic(err)
	}

	defer term.Restore(int(os.Stdin.Fd()), oldState)

	if !blVerbose {
		MENU[9].Name = strings.Replace(MENU[9].Name, "ON", "OFF", 1)
	} else {
		MENU[9].Name = strings.Replace(MENU[9].Name, "OFF", "ON", 1)
	}
	// Print menu
	fmt.Println("\nSelect an option:")

	for _, option := range MENU[1:] {
		fmt.Printf("\t[%c] %s\n", option.Input, option.Name)
	}
	// Receive option from user
	var pSelectedOpt Option = MENU[0]

	for pSelectedOpt == MENU[0] {

		keyPressed := GetKeyStroke()

		if keyPressed == rune(27) || keyPressed == '0' || keyPressed == 'q' || keyPressed == 'Q' { // Quit
			return false
		}

		for i := range MENU {

			if MENU[i].Input == keyPressed {

				pSelectedOpt = MENU[i]
				break
			}
		}
	}

	switch pSelectedOpt.Input {

	case 'h', 'H':
		{
			ShowHelp()
			return true
		}

	case 'v', 'V':
		{
			blVerbose = !blVerbose

			if !blVerbose {
				MENU[9].Name = strings.Replace(MENU[8].Name, "ON", "OFF", 1)
			} else {
				MENU[9].Name = strings.Replace(MENU[8].Name, "OFF", "ON", 1)
			}

			ClearTerminal()

			return true
		}

	default:
		fmt.Printf(ColouriseText("Selected: %s\n", Blue, ""), pSelectedOpt.Name)
	}
	// Move terminal restore BEFORE Scanln
	term.Restore(int(os.Stdin.Fd()), oldState)
	chosenBSP := GetPromptInput("Drag a BSP file or folder you want to ripent (leave blank to use the current folder, enter 'x' to cancel):")

	if chosenBSP == "x" {
		return true
	}

	if pSelectedOpt.Argument == "-edit" && strings.HasSuffix(chosenBSP, ".bsp") {

		if STR_EXES[1] == "" {

			fmt.Printf(ColouriseText("Warning: Lazyripent is not installed.\n%s requires Lazyripent to work. Please download and install and Lazyripent then launch the application again", Yellow, ""), pSelectedOpt.Name)
			return true
		}

		LaunchEditor(chosenBSP)
		return true
	}

	var rule *string

	if pSelectedOpt.Argument == "-rule" {

		if STR_EXES[1] == "" {

			fmt.Printf(ColouriseText("Warning: Lazyripent is not installed.\n%s requires Lazyripent to work. Please download and install and Lazyripent then launch the application again", Yellow, ""), pSelectedOpt.Name)
			return true
		}

		input := GetPromptInput("Drag rule file or folder (leave blank to use the current folder, enter 'x' to cancel):")
		rule = &input

		if *rule == "x" {
			return true
		}
	}
	// Re-enter raw mode for next iteration
	_, err = term.MakeRaw(int(os.Stdin.Fd()))

	if err != nil {
		// Something bad happened, panic
		fmt.Println(ColouriseText("\n\t(╯°□°)╯︵ ┻━┻\t\nPANIC - something went wrong.\n", Red, ""))
		fmt.Printf(ColouriseText("Table Flippation: %s", Red, ""), err)
		bufio.NewReader(os.Stdin).ReadBytes('\n')
	}

	if rule == nil {
		RipEntities(chosenBSP, pSelectedOpt.Argument, blVerbose)
	} else {
		ApplyRule(*rule, chosenBSP)
	}

	return true
}

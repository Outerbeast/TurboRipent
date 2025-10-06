/*
	TurboRipent - TUI Frontend for Ripent / Lazyripent
	Version 1.1

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
	"errors"
	"fmt"
	"io/fs"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"
)

func GetBSPS(strInput string) []string {

	var BSPS []string
	normalizedPath := filepath.Clean(filepath.Join(strInput, ""))
	info, err := os.Stat(normalizedPath)

	if err != nil && !errors.Is(err, fs.ErrNotExist) {
		LoudPanic("Something went wrong.", err)
	}

	var scanPath string

	if err == nil && info.IsDir() {
		scanPath = normalizedPath
	} else if strInput == "" {
		scanPath = "."
	}

	if scanPath != "" {

		ENTRIES, err := os.ReadDir(scanPath)

		if err != nil {
			LoudPanic("Something went wrong.", err)
		}

		for _, entry := range ENTRIES {

			name := entry.Name()

			if entry.Type().IsRegular() && strings.HasSuffix(name, ".bsp") {
				BSPS = append(BSPS, name)
			}
		}
	} else if strings.HasSuffix(strInput, "*") {

		prefix := filepath.Base(normalizedPath)
		entries, err := os.ReadDir(filepath.Dir(normalizedPath))

		if err != nil {
			LoudPanic("Something went wrong.", err)
		}

		for _, entry := range entries {

			name := entry.Name()

			if entry.Type().IsRegular() && strings.HasPrefix(name, prefix) && strings.HasSuffix(name, ".bsp") {
				BSPS = append(BSPS, name)
			}
		}
	} else {
		BSPS = append(BSPS, filepath.Base(normalizedPath))
	}

	return BSPS
}

func RipEntities(strBspName string, strArg string, blVerbose bool) {

	if STR_EXES[0] == "" {
		LoudPanic("Ripent is not installed.", nil)
	} else {
		fmt.Printf(ColouriseText("Executing %s: '%s'\n", Grey, ""), strArg, STR_EXES[0])
	}

	var BSPS []string = GetBSPS(strBspName)

	if len(BSPS) < 1 {
		fmt.Printf("%s", ColouriseText("⚠️ No BSP files were processed.\n", Yellow, ""))
		return
	}

	var countSuccess, countFail byte

	for _, bsp := range BSPS {
		fmt.Printf(ColouriseText("Processing BSP: %s\n", Cyan, ""), bsp)

		// Get the absolute path for the BSP file
		var BSPPath string
		if filepath.Ext(strBspName) == ".bsp" || len(BSPS) == 1 {
			BSPPath = bsp
		} else {
			BSPPath = filepath.Join(strBspName, bsp)
		}

		absBSPPath, err := filepath.Abs(BSPPath)

		if err != nil {

			fmt.Printf(ColouriseText("⚠️ Error getting absolute path for %s: %s\n", Yellow, ""), BSPPath, err)
			continue
		}

		cmdRipent := exec.Command(STR_EXES[0], strArg, absBSPPath)

		if strArg == "-chart" {

			fileChart, err := os.OpenFile(absBSPPath+".log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)

			if err != nil {
				log.Fatal(err)
			}

			defer fileChart.Close()
			cmdRipent.Stdout = fileChart
		}

		// Output to console
		if blVerbose {
			cmdRipent.Stdout = os.Stdout
			cmdRipent.Stderr = os.Stderr
		}

		err = cmdRipent.Run()

		if err != nil {

			fmt.Printf(ColouriseText("❌ Error processing %s: %s\n", Red, ""), absBSPPath, err)
			countFail++
		} else {
			// For imports, remove imported .ent files
			if strArg == "-import" {

				ent := strings.TrimSuffix(BSPPath, filepath.Ext(bsp)) + ".ent"
				absENTPath, err := filepath.Abs(ent)

				if err != nil {
					fmt.Printf(ColouriseText("⚠️ Error getting absolute path for %s: %s\n", Yellow, ""), ent, err)
					continue
				}

				if os.Remove(absENTPath) != nil {
					fmt.Println(ColouriseText("⚠️ Couldn't delete: ", Yellow, ""), absENTPath)
				}
			}

			countSuccess++
		}
	}

	if countSuccess > 0 {
		fmt.Printf(ColouriseText("✅ %v BSP(s) processed.\n", Green, ""), countSuccess)
	}

	if countFail > 0 {
		fmt.Printf(ColouriseText("⚠️ %v BSP(s) failed to process.\nCheck that the .ent file exists for the BSP if importing.\n", Yellow, ""), countFail)
	}
}

/*
!-TODO-!: Extraction/Imports entities in JSON format via Lazyripent
lazyripent extracts JSON format entity files in .ent file extension rather than .json.
To do a file extension change after extraction can cause blast radius on pre-exisiting .ent files extracted via stock Ripent.exe beforehand.
Similarly, cleanup on imported ent files can remove ent files that weren't necessarily imported by Lazyripent
Lazyripent should natively support options for:-
- Export ent files with the .json file extension
- Clean up imported .json files (deletion)
*/
func RipJSON(input string, shouldImport bool, skipConfirm bool) {

	var ARGUMENTS []string

	if strings.HasSuffix(input, ".bsp") {

		input = strings.ReplaceAll(input, "\"", "")
		entOutput := strings.TrimSuffix(input, ".bsp") + ".ent"

		if !shouldImport {
			fmt.Printf("Extracting ents from bsp file: %s to %s\n", input, entOutput)
			ARGUMENTS = []string{"-i", input, "-o", entOutput, "-ee"}
		} else {
			fmt.Printf("Importing entity data from ent file to bsp: %s to %s\n", input, entOutput)
			ARGUMENTS = []string{"-i", input, "-i", entOutput, "-o", input, "-ie"}
		}

	} else { // Using existing folder

		if input == "" {
			input, _ = os.Getwd()
		}

		if !shouldImport {
			fmt.Printf("Extracting entity data from bsp files in the folder: %s\n", input)
			ARGUMENTS = []string{"-i", input, "-o", input, "-ee"}
		} else {
			fmt.Printf("Importing entity data from ent files in the folder: %s\n", input)
			ARGUMENTS = []string{"-i", input, "-o", strings.TrimSuffix(input, ".ent") + ".bsp", "-ie"}
		}
	}

	if skipConfirm {
		ARGUMENTS = append(ARGUMENTS, "-u")
	}

	ExecLazyripent(ARGUMENTS)
}

func ApplyRule(ruleFile string, output string) {

	var RULES, ARGUMENTS []string

	if output == "" {
		output = strCurrentPath
	}

	if strings.HasSuffix(ruleFile, ".rule") {

		fmt.Printf(ColouriseText("Using rule file: %s", Cyan, ""), ruleFile)
		ARGUMENTS = []string{"-i", ruleFile, "-i", output, "-o", output, "-u"}
	} else { // its a folder

		if ruleFile == "" {
			ruleFile = strCurrentPath
		}

		fmt.Println(ColouriseText("Using rule files from folder:", Cyan, ""), ruleFile)

		filepath.Walk(strCurrentPath, func(path string, info os.FileInfo, err error) error {

			if err != nil {
				return err
			}

			if strings.HasSuffix(info.Name(), ".rule") {

				if slices.Contains(RULES, path) {
					return nil
				}

				RULES = append(RULES, path)
			}

			return nil
		})

		if len(RULES) > 0 {

			var countSuccess, countFail byte

			for _, rule := range RULES {

				if ExecLazyripent([]string{"-i", rule, "-i", output, "-o", output, "-u"}) {
					countSuccess++
				} else {
					countFail++
				}
			}

			if countSuccess > 0 {
				fmt.Printf(ColouriseText("✅ %v rules processed.\n", Green, ""), countSuccess)
			}

			if countFail > 0 {
				fmt.Printf(ColouriseText("⚠️ %v rules failed to process.\n", Yellow, ""), countFail)
			}

		} else {
			fmt.Println("No rule files were found. Skipping...")
		}

		return
	}

	if !ExecLazyripent(ARGUMENTS) {
		fmt.Printf(ColouriseText("⚠️ Rule '%s' failed to apply to '%s'.\n", Yellow, ""), ruleFile, output)
	} else {
		fmt.Printf(ColouriseText("✅ Rule '%s' rule applied to '%s'.\n", Green, ""), ruleFile, output)
	}
}

func ExecLazyripent(ARGUMENTS []string) bool {

	if len(ARGUMENTS) < 1 {
		return false
	}

	fmt.Printf(ColouriseText("Executing %s: '%v'\n", Grey, ""), STR_EXES[1], ARGUMENTS)
	cmdLazyripent := exec.Command(STR_EXES[1], ARGUMENTS...)
	cmdLazyripent.Stdin = os.Stdin // Attach console input
	cmdLazyripent.Stdout = os.Stdout
	cmdLazyripent.Stderr = os.Stderr

	err := cmdLazyripent.Run()

	if err != nil {

		if errors.Is(err, os.ErrNotExist) {
			// Lazyripent is absent, throw error (and table)
			fmt.Println(ColouriseText("\n\t(╯°□°)╯︵ ┻━┻\t\nLazyripent is not installed.\nPlease install Lazyripent in order to apply rules or use the editor.\n", Red, ""))
			fmt.Printf(ColouriseText("Table Flippation: %s", Red, ""), err)
			bufio.NewReader(os.Stdin).ReadBytes('\n')
		}

		fmt.Println(ColouriseText("❌ Error processing lazyripent:", Red, ""), err)

		return false
	} else {
		return true
	}
}

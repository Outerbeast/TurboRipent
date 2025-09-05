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
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"maps"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
)

const ConfFileName string = AppName + "_conf.json"

var (
	blVerbose      bool = false
	strCurrentPath string
	// Executable filenames here get prepended with the path once they are found.
	STR_EXES                 []string          = []string{map[bool]string{true: "Ripent_x64.exe", false: "Ripent.exe"}[runtime.GOARCH == "amd64"], "lazyripent.exe"}
	mapDefaultEntityTemplate map[string]string // template entity for the editor "Create" button
)

type Config struct {
	RipentPath     string            `json:"RipentPath"`
	LazyripentPath string            `json:"LazyripentPath"`
	Verbose        bool              `json:"Verbose"`
	DefaultEntity  map[string]string `json:"DefaultEntity"`
}

func GetConfigPath() string {
	return os.Getenv("LOCALAPPDATA")
}

func LoadConfig(path string) (*Config, error) {

	data, err := os.ReadFile(path)

	if err != nil {
		return nil, err
	}

	var cfg Config

	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}

	return &cfg, nil
}

func SaveConfig(path string, cfg *Config) error {

	data, err := json.MarshalIndent(cfg, "", "  ")

	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}

func Init() {

	exePath, err := os.Executable()

	if err != nil {
		log.Fatalf(ColouriseText("Failed to get executable path: %v", Red, ""), err)
	}

	strCurrentPath = filepath.Dir(exePath)
	cfgLoad, err := LoadConfig(GetConfigPath() + "\\" + ConfFileName)

	if err != nil {

		if errors.Is(err, os.ErrNotExist) {
			// Run searches concurrently
			var RipentExists, LazyripentExists bool
			var wg sync.WaitGroup
			wg.Add(2)
			fmt.Println(ColouriseText("Initial setup, please wait...\n", Grey, ""))

			go func() {

				defer wg.Done()
				RipentExists = SearchInstall(&STR_EXES[0])
			}()

			go func() {

				defer wg.Done()
				LazyripentExists = SearchInstall(&STR_EXES[1])
			}()

			wg.Wait()

			if !RipentExists && !LazyripentExists {

				fmt.Println(ColouriseText("Ripent executable not found.\nPlease manually set the path to your Sven Co-op addons folder, or reinstall Sven Co-op SDK and try again.", Red, ""))
				fmt.Print("\nPress Enter to exit...")
				fmt.Scanln() // waits for user input
				os.Exit(1)
			}

			if !LazyripentExists {
				STR_EXES[1] = ""
			}

			StoreSettings()

		} else if errors.Is(err, os.ErrPermission) {
			fmt.Println(ColouriseText("Permission error: ", Red, ""), err)
		}
	} else {

		STR_EXES[0] = strings.TrimSpace(cfgLoad.RipentPath)
		STR_EXES[1] = strings.TrimSpace(cfgLoad.LazyripentPath)
		blVerbose = cfgLoad.Verbose

		if len(cfgLoad.DefaultEntity) > 0 {

			mapDefaultEntityTemplate = make(map[string]string, len(cfgLoad.DefaultEntity))
			maps.Copy(mapDefaultEntityTemplate, cfgLoad.DefaultEntity)
		} else {
			mapDefaultEntityTemplate = nil
		}
	}
}

func SearchInstall(exeName *string) bool {

	if exeName == nil {

		fmt.Println(ColouriseText("ERROR - value exeName is nil", Red, ""))
		return false
	}

	for _, drive := range getDrives() {

		MATCHES := findFileOnDrive(drive, *exeName)

		for _, match := range MATCHES {

			if match == "" {
				continue
			}

			fullPath := filepath.Join(filepath.Dir(match), *exeName)

			if _, err := os.Stat(fullPath); err == nil {

				*exeName = fullPath
				return true
			}

			return true
		}
	}

	return false
}

func StoreSettings() {

	cfg := &Config{
		RipentPath:     STR_EXES[0],
		LazyripentPath: STR_EXES[1],
		Verbose:        blVerbose,
		DefaultEntity:  mapDefaultEntityTemplate,
	}

	if mapDefaultEntityTemplate == nil {
		// Just in case defaulty entity template is blank
		mapDefaultEntityTemplate = map[string]string{
			"classname":  "info_null",
			"origin":     "0 0 0",
			"angles":     "0 0 0",
			"spawnflags": "0",
		}
	}

	if err := SaveConfig(filepath.Join(GetConfigPath(), ConfFileName), cfg); err != nil {
		panic(err)
	}
}

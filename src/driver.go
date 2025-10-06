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
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"
)

const AppName string = "TurboRipent"

func main() {

	enableANSIColors()
	SetConsoleTitle(AppName)
	fmt.Printf("%s\n%s\n\n", ColouriseText(AppName, "", BgGreen), "Extract and Import BSP entity data")
	Init()
	// Save on normal exit
	defer StoreSettings()
	// Save on Ctrl+C / SIGTERM
	c := make(chan os.Signal, 1)
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-c
		StoreSettings()
		os.Exit(0)
	}()

	if len(os.Args) > 1 {

		if os.Args[1] == "-edit" {
			LaunchEditor(os.Args[2])
		} else {
			// Handle quick extract/import/rule
			for _, path := range os.Args[1:] {

				if !strings.HasSuffix(path, ".bsp") &&
					!strings.HasSuffix(path, ".ent") &&
					!strings.HasSuffix(path, ".rule") {
					continue
				}

				if strings.HasSuffix(path, ".bsp") {
					RipEntities(path, "-export", false)
				} else if strings.HasSuffix(path, ".ent") {
					path = strings.TrimSuffix(path, ".ent") + ".bsp"
					RipEntities(path, "-import", false)
				} else if strings.HasSuffix(path, ".rule") {
					ApplyRule(path, strings.TrimSuffix(path, ".rule")+".bsp")
				}
			}
		}
	} else {

		for {

			if !DisplayMenu() {
				break
			}
		}
	}
}

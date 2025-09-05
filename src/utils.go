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
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"unsafe"

	"golang.org/x/sys/windows"
)

const (
	Reset         = "\033[0m"
	Red           = "\033[31m"
	Green         = "\033[32m"
	Yellow        = "\033[33m"
	Blue          = "\033[34m"
	Magenta       = "\033[35m"
	Cyan          = "\033[36m"
	White         = "\033[37m"
	Grey          = "\033[90m"
	BrightRed     = "\033[91m"
	BrightGreen   = "\033[92m"
	BrightYellow  = "\033[93m"
	BrightBlue    = "\033[94m"
	BrightMagenta = "\033[95m"
	BrightCyan    = "\033[96m"
	BrightWhite   = "\033[97m"

	// Background colors
	BgRed           = "\033[41m"
	BgGreen         = "\033[42m"
	BgYellow        = "\033[43m"
	BgBlue          = "\033[44m"
	BgMagenta       = "\033[45m"
	BgCyan          = "\033[46m"
	BgWhite         = "\033[47m"
	BgGrey          = "\033[100m"
	BgBrightRed     = "\033[101m"
	BgBrightGreen   = "\033[102m"
	BgBrightYellow  = "\033[103m"
	BgBrightBlue    = "\033[104m"
	BgBrightMagenta = "\033[105m"
	BgBrightCyan    = "\033[106m"
	BgBrightWhite   = "\033[107m"
)

func ColouriseText(text, fg, bg string) string {
	return fg + bg + text + Reset
}

func SetConsoleTitle(title string) error {

	kernel32 := windows.NewLazySystemDLL("kernel32.dll")
	setConsoleTitleW := kernel32.NewProc("SetConsoleTitleW")
	_, _, err := setConsoleTitleW.Call(uintptr(unsafe.Pointer(windows.StringToUTF16Ptr(title))))

	if err != nil && err.Error() != "The operation completed successfully." {
		return err
	}

	return nil
}

func enableANSIColors() {
	// Get handle to STDOUT
	kernel32 := syscall.NewLazyDLL("kernel32.dll")
	getStdHandle := kernel32.NewProc("GetStdHandle")
	setConsoleMode := kernel32.NewProc("SetConsoleMode")
	getConsoleMode := kernel32.NewProc("GetConsoleMode")

	const STD_OUTPUT_HANDLE = uint32(-11 & 0xFFFFFFFF)
	hOut, _, _ := getStdHandle.Call(uintptr(STD_OUTPUT_HANDLE))

	if hOut == 0 {
		return
	}

	var mode uint32
	getConsoleMode.Call(hOut, uintptr(unsafe.Pointer(&mode)))

	// Enable ANSI escape sequence processing
	mode |= 0x0004
	setConsoleMode.Call(hOut, uintptr(mode))
}

func setHidden(path string) error {

	ptr, err := syscall.UTF16PtrFromString(path)

	if err != nil {
		return err
	}
	// FILE_ATTRIBUTE_HIDDEN = 0x2
	return syscall.SetFileAttributes(ptr, syscall.FILE_ATTRIBUTE_HIDDEN)
}

func getDrives() []string {

	var DRIVES []string

	for d := 'A'; d <= 'Z'; d++ {

		path := fmt.Sprintf("%c:\\", d)

		if _, err := os.Stat(path); err == nil {
			DRIVES = append(DRIVES, path)
		}
	}

	return DRIVES
}

func findFileOnDrive(drive string, targetFile string) []string {

	var MATCHES []string

	filepath.WalkDir(drive, func(path string, d fs.DirEntry, err error) error {

		if err != nil {
			return nil // skip inaccessible paths
		}

		if d.IsDir() {

			base := strings.ToLower(d.Name())
			// If this is the recycle bin folder, skip it entirely
			if base == "$recycle.bin" || base == "recycler" {
				return fs.SkipDir
			}
		}

		if !d.IsDir() && strings.EqualFold(filepath.Base(path), targetFile) {
			MATCHES = append(MATCHES, path)
		}

		return nil
	})

	return MATCHES
}

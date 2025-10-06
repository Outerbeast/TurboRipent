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
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"unsafe"

	"golang.org/x/sys/windows"
)

var (
	dllKernel32 = windows.NewLazySystemDLL("kernel32.dll")

	procSetConsoleTitleW = dllKernel32.NewProc("SetConsoleTitleW")
	procGetConsoleWindow = dllKernel32.NewProc("GetConsoleWindow")
	procShowWindow       = dllUser32.NewProc("ShowWindow")
	procGetStdHandle     = dllKernel32.NewProc("GetStdHandle")
	procGetConsoleMode   = dllKernel32.NewProc("GetConsoleMode")
	procSetConsoleMode   = dllKernel32.NewProc("SetConsoleMode")
)

// Windows ShowWindow constants
const (
	SW_HIDE           = 0
	SW_SHOWNORMAL     = 1
	SW_SHOWMINIMIZED  = 2
	SW_SHOWMAXIMIZED  = 3
	SW_SHOWNOACTIVATE = 4
	SW_SHOW           = 5
	SW_MINIMIZE       = 6
	SW_RESTORE        = 9
)

const (
	STD_OUTPUT_HANDLE                  = uintptr(^uint32(10) + 1) // -11 as uintptr
	ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004
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

// SetConsoleTitle sets the console window title.
func SetConsoleTitle(title string) error {

	ret, _, err := procSetConsoleTitleW.Call(
		uintptr(unsafe.Pointer(windows.StringToUTF16Ptr(title))),
	)

	if ret == 0 {
		// API failed, err contains the Windows error
		return err
	}

	return nil
}

// hideConsole hides the console window (or minimizes if you swap SW_HIDE for SW_MINIMIZE).
func hideConsole() error {

	hwnd, _, _ := procGetConsoleWindow.Call()

	if hwnd == 0 {
		return fmt.Errorf("no console window handle")
	}
	// ShowWindow returns the previous visibility state, not strictly an error
	procShowWindow.Call(hwnd, SW_HIDE)
	return nil
}

// enableANSIColors enables ANSI escape sequence processing in the Windows console.
func enableANSIColors() error {
	// Get handle to STDOUT
	hOut, _, err := procGetStdHandle.Call(STD_OUTPUT_HANDLE)

	if hOut == 0 {
		return fmt.Errorf("GetStdHandle failed: %v", err)
	}

	var mode uint32
	ret, _, err := procGetConsoleMode.Call(hOut, uintptr(unsafe.Pointer(&mode)))

	if ret == 0 {
		return fmt.Errorf("GetConsoleMode failed: %v", err)
	}

	// Enable ANSI escape sequence processing
	mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING
	ret, _, err = procSetConsoleMode.Call(hOut, uintptr(mode))

	if ret == 0 {
		return fmt.Errorf("SetConsoleMode failed: %v", err)
	}

	return nil
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

// !-TODO-!: Proper error handling - for now this is a hibernating bear
func wtfPointer(s string) *uint16 {

	ptr, err := syscall.UTF16PtrFromString(s)

	if err != nil {
		LoudPanic("Fatal Error: wtfPointer assign failed.", err)
	}

	return ptr
}

func LoudPanic(msg string, e error) {

	fmt.Println(ColouriseText("\n\t(╯°□°)╯︵ ┻━┻\t\n%s\n", Red, ""), msg)
	fmt.Printf(ColouriseText("Table Flippation: %s", Red, ""), e)
	panic(e)
}

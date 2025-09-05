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
	"fmt"
	"maps"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
	"syscall"
	"unsafe"

	"github.com/lxn/win"
)

var (
	hInstance                                                 win.HINSTANCE
	listBox, textBox, bntSave, btnCreate, btnClone, btnDelete win.HWND
	ENTITIES                                                  []Entity
	strEntityFile                                             string
	blUpdatingListbox                                         bool
	pClassName                                                *uint16
	registerUI                                                sync.Once
	user32                                                    = syscall.NewLazyDLL("user32.dll")
	procGetWindowTextW                                        = user32.NewProc("GetWindowTextW")
	procGetWindowTextLengthW                                  = user32.NewProc("GetWindowTextLengthW")
	procSetWindowTextW                                        = user32.NewProc("SetWindowTextW")
	CloseMode                                                 = closeNone
)

const (
	windowHeight     = 900
	windowWidth      = 700
	windowHeightMin  = 600
	windowWidthMin   = 400
	WM_SAVE_COMPLETE = win.WM_USER + 1 // Guard to suppress re-entrant LBN_SELCHANGE when we programmatically mutate the listbox.
	// Editor close flags
	closeNone   = iota
	closeSilent // Save button: apply with -u
	closePrompt // X-close: prompt
)

type Entity struct {
	KeyValues map[string]string `json:"KeyValues"`
}

// Entity business
func loadEntities(path string) ([]Entity, error) {

	B, err := os.ReadFile(path)

	if err != nil {
		return nil, err
	}

	var OUT []Entity

	if err := json.Unmarshal(B, &OUT); err != nil {
		return nil, err
	}

	return OUT, nil
}

func classnamesFromEntities(ENTITIES []Entity) []string {

	NAMES := make([]string, 0, len(ENTITIES))

	for _, entity := range ENTITIES {

		if cn, ok := entity.KeyValues["classname"]; ok && cn != "" {
			NAMES = append(NAMES, cn)
		} else {
			NAMES = append(NAMES, "<no classname>")
		}
	}

	return NAMES
}

func renderKeyValues(kv map[string]string) string {

	if kv == nil {
		return ""
	}

	KEYS := make([]string, 0, len(kv))

	for k := range kv {
		KEYS = append(KEYS, k)
	}

	sort.Strings(KEYS)
	out := ""

	for i, k := range KEYS {

		out += fmt.Sprintf("%s=%s", k, kv[k])

		if i != len(KEYS)-1 {
			out += "\r\n"
		}
	}

	return out
}

func parseKeyValues(s string) map[string]string {

	kv := make(map[string]string)
	LINES := splitLines(s)

	for _, line := range LINES {

		line = strings.TrimSpace(line)

		if line == "" {
			continue
		}
		// Split only on the first '='
		if eq := strings.IndexRune(line, '='); eq >= 0 {

			key := strings.TrimSpace(line[:eq])
			val := strings.TrimSpace(line[eq+1:])
			kv[key] = val
		}
	}

	return kv
}

func splitLines(s string) []string {
	// Handle CRLF and LF
	OUT := []string{}
	start := 0

	for i := 0; i < len(s); i++ {

		if s[i] == '\n' {

			line := s[start:i]

			if len(line) > 0 && line[len(line)-1] == '\r' {
				line = line[:len(line)-1]
			}

			OUT = append(OUT, line)
			start = i + 1
		}
	}

	if start < len(s) {
		OUT = append(OUT, s[start:])
	}

	return OUT
}

// Snapshot to avoid data races during async save
func snapshotEntities(in []Entity) []Entity {

	OUT := make([]Entity, len(in))

	for i := range in {

		OUT[i].KeyValues = make(map[string]string, len(in[i].KeyValues))

		maps.Copy(OUT[i].KeyValues, in[i].KeyValues)
	}

	return OUT
}

func saveEntities(path string, entities []Entity) error {

	data, err := json.MarshalIndent(entities, "", "  ")

	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}

// Window business
func getWindowText(hwnd win.HWND) string {

	textLen, _, _ := procGetWindowTextLengthW.Call(uintptr(hwnd))

	if textLen == 0 {
		return ""
	}

	buf := make([]uint16, textLen+1)
	procGetWindowTextW.Call(
		uintptr(hwnd),
		uintptr(unsafe.Pointer(&buf[0])),
		uintptr(len(buf)),
	)
	return syscall.UTF16ToString(buf)
}

func setWindowText(hwnd win.HWND, s string) {
	ptr, _ := syscall.UTF16PtrFromString(s)
	procSetWindowTextW.Call(uintptr(hwnd), uintptr(unsafe.Pointer(ptr)))
}

func LaunchEditor(chosenBSP string) {
	// No BSP - Ask user to manually give a BSP
	if chosenBSP == "" || !strings.HasSuffix(chosenBSP, ".bsp") {

		chosenBSP = GetPromptInput("Drag a BSP file you want to edit (enter 'x' to cancel):")

		if chosenBSP == "" || chosenBSP == "x" || !strings.HasSuffix(chosenBSP, ".bsp") {
			return
		}
	}

	fmt.Printf(ColouriseText("Opening: %s\n", Grey, ""), chosenBSP)
	RipJSON(chosenBSP, false, true) // skip confirmation as a previous entity file should automatically be overwritten
	strEntityFile = strings.TrimSuffix(chosenBSP, ".bsp") + ".ent"
	setHidden(strEntityFile)

	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	hInstance = win.GetModuleHandle(nil)
	pClassName, _ = syscall.UTF16PtrFromString("MyWin32WindowClass")

	// wrap RegisterClassEx in sync.Once so it runs only the first time ---
	registerUI.Do(func() {
		var wc win.WNDCLASSEX
		wc.CbSize = uint32(unsafe.Sizeof(wc))
		wc.LpfnWndProc = syscall.NewCallback(EditorWindow)
		wc.HInstance = hInstance
		wc.HCursor = win.LoadCursor(0, win.MAKEINTRESOURCE(win.IDC_ARROW))
		wc.HbrBackground = win.HBRUSH(win.COLOR_WINDOW + 1)
		wc.LpszClassName = pClassName

		if win.RegisterClassEx(&wc) == 0 {
			panic(fmt.Sprintf("RegisterClassEx failed: %v", syscall.GetLastError()))
		}
	})

	hwnd := win.CreateWindowEx(
		0,
		pClassName,
		syscall.StringToUTF16Ptr(AppName+" Editor - "+filepath.Base(chosenBSP)),
		win.WS_OVERLAPPEDWINDOW,
		win.CW_USEDEFAULT, win.CW_USEDEFAULT,
		windowHeight, windowWidth,
		0, 0, hInstance, nil,
	)

	win.ShowWindow(hwnd, win.SW_SHOWDEFAULT)
	win.UpdateWindow(hwnd)

	var msg win.MSG

	for win.GetMessage(&msg, 0, 0, 0) > 0 {
		win.TranslateMessage(&msg)
		win.DispatchMessage(&msg)
	}
}

// Main Window
func EditorWindow(hwnd win.HWND, msg uint32, wParam, lParam uintptr) uintptr {

	switch msg {

	case win.WM_CREATE:
		{
			// ListBox (explicit styles; no LBS_STANDARD/LBS_SORT)
			listBox = win.CreateWindowEx(
				0, syscall.StringToUTF16Ptr("LISTBOX"), nil,
				win.WS_CHILD|win.WS_VISIBLE|win.WS_BORDER|win.WS_VSCROLL|win.LBS_NOTIFY,
				10, 10, 200, 400,
				hwnd, 0, hInstance, nil,
			)

			// Multiline TextBox
			textBox = win.CreateWindowEx(
				win.WS_EX_CLIENTEDGE, syscall.StringToUTF16Ptr("EDIT"), nil,
				win.WS_CHILD|win.WS_VISIBLE|win.WS_BORDER|win.WS_VSCROLL|
					win.ES_AUTOVSCROLL|win.ES_MULTILINE|win.ES_WANTRETURN,
				220, 10, 450, 360,
				hwnd, 0, hInstance, nil,
			)

			// Buttons
			btnCreate = win.CreateWindowEx(0,
				syscall.StringToUTF16Ptr("BUTTON"),
				syscall.StringToUTF16Ptr("Create"),
				win.WS_CHILD|win.WS_VISIBLE,
				0, 0, 80, 30, hwnd, 0, hInstance, nil)

			btnClone = win.CreateWindowEx(0,
				syscall.StringToUTF16Ptr("BUTTON"),
				syscall.StringToUTF16Ptr("Clone"),
				win.WS_CHILD|win.WS_VISIBLE,
				0, 0, 80, 30, hwnd, 0, hInstance, nil)

			btnDelete = win.CreateWindowEx(0,
				syscall.StringToUTF16Ptr("BUTTON"),
				syscall.StringToUTF16Ptr("Delete"),
				win.WS_CHILD|win.WS_VISIBLE,
				0, 0, 80, 30, hwnd, 0, hInstance, nil)

			bntSave = win.CreateWindowEx(0,
				syscall.StringToUTF16Ptr("BUTTON"),
				syscall.StringToUTF16Ptr("Save"),
				win.WS_CHILD|win.WS_VISIBLE|win.BS_DEFPUSHBUTTON,
				0, 0, 80, 30, hwnd, 0, hInstance, nil)

			// Load entities
			var err error
			ENTITIES, err = loadEntities(strEntityFile)
			if err != nil {
				setWindowText(textBox, fmt.Sprintf("Error loading entities.json:\r\n%v", err))
				return 0
			}

			// Populate listbox with redraw suppression
			setRedraw(listBox, false)
			for _, name := range classnamesFromEntities(ENTITIES) {
				win.SendMessage(listBox, win.LB_ADDSTRING, 0,
					uintptr(unsafe.Pointer(utf16Ptr(name))))
			}
			setRedraw(listBox, true)
		}

	case win.WM_GETMINMAXINFO:
		{
			mmi := (*win.MINMAXINFO)(unsafe.Pointer(lParam))
			mmi.PtMinTrackSize.X = windowHeightMin // min width
			mmi.PtMinTrackSize.Y = windowWidthMin  // min height
			return 0
		}

	case win.WM_SIZE:
		{
			width := int(win.LOWORD(uint32(lParam)))
			height := int(win.HIWORD(uint32(lParam)))

			margin := 10
			buttonWidth := 80
			buttonHeight := 30
			buttonSpacing := 10

			// Height for listbox and textbox so they match
			listHeight := height - (margin*2 + buttonHeight + buttonSpacing)

			// ListBox on the left
			win.MoveWindow(listBox, int32(margin), int32(margin), 200, int32(listHeight), true)

			// TextBox to the right of listbox
			textX := margin + 200 + margin
			textWidth := width - textX - margin
			win.MoveWindow(textBox, int32(textX), int32(margin), int32(textWidth), int32(listHeight), true)

			// Buttons row Y position
			btnY := margin + listHeight + buttonSpacing

			// Create aligned to left edge of textbox
			createX := textX
			win.MoveWindow(btnCreate, int32(createX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Clone to the right of Create
			cloneX := createX + buttonWidth + buttonSpacing
			win.MoveWindow(btnClone, int32(cloneX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Delete to the right of Clone
			deleteX := cloneX + buttonWidth + buttonSpacing
			win.MoveWindow(btnDelete, int32(deleteX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Save anchored to far right
			saveX := width - margin - buttonWidth
			win.MoveWindow(bntSave, int32(saveX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)
		}

	case win.WM_COMMAND:
		{
			notify := win.HIWORD(uint32(wParam))
			src := win.HWND(lParam)

			// Selection change: ignore while we're programmatically updating the listbox
			if src == listBox && notify == win.LBN_SELCHANGE && !blUpdatingListbox {
				sel := int(win.SendMessage(listBox, win.LB_GETCURSEL, 0, 0))
				if sel >= 0 && sel < len(ENTITIES) {
					setWindowText(textBox, renderKeyValues(ENTITIES[sel].KeyValues))
				}
			}

			// Save (async; snapshot to avoid races)
			if src == bntSave && notify == win.BN_CLICKED {
				CloseMode = closeSilent // declare intent
				sel := int(win.SendMessage(listBox, win.LB_GETCURSEL, 0, 0))
				if sel >= 0 && sel < len(ENTITIES) {
					ENTITIES[sel].KeyValues = parseKeyValues(getWindowText(textBox))
					snap := snapshotEntities(ENTITIES)
					go func(hwnd win.HWND, ents []Entity, sel int) {
						err := saveEntities(strEntityFile, ents)
						status := uintptr(0)
						if err != nil {
							status = 1
						}
						win.PostMessage(hwnd, WM_SAVE_COMPLETE, status, uintptr(sel))
					}(hwnd, snap, sel)
				}
			}

			// Create: guarded UI mutation + async persist
			if src == btnCreate && notify == win.BN_CLICKED {
				var kv map[string]string
				if len(mapDefaultEntityTemplate) > 0 {
					// Deep copy template so edits don't mutate the config
					kv = make(map[string]string, len(mapDefaultEntityTemplate))
					for k, v := range mapDefaultEntityTemplate {
						kv[k] = v
					}
				} else {
					kv = map[string]string{"classname": "new_entity"}
				}

				newEntity := Entity{KeyValues: kv}
				ENTITIES = append(ENTITIES, newEntity)
				name := newEntity.KeyValues["classname"]
				if name == "" {
					name = "<no classname>"
				}

				blUpdatingListbox = true
				setRedraw(listBox, false)
				idx := int(win.SendMessage(listBox, win.LB_ADDSTRING, 0, uintptr(unsafe.Pointer(utf16Ptr(name)))))
				win.SendMessage(listBox, win.LB_SETCURSEL, uintptr(idx), 0)
				win.SendMessage(listBox, win.LB_SETTOPINDEX, uintptr(idx), 0)
				setRedraw(listBox, true)
				blUpdatingListbox = false

				setWindowText(textBox, renderKeyValues(newEntity.KeyValues))

				snap := snapshotEntities(ENTITIES)
				go func(hwnd win.HWND, ents []Entity, sel int) {
					err := saveEntities(strEntityFile, ents)
					status := uintptr(0)
					if err != nil {
						status = 1
					}
					win.PostMessage(hwnd, WM_SAVE_COMPLETE, status, uintptr(sel))
				}(hwnd, snap, idx)
			}

			// Clone: duplicate selected entity + async persist
			if src == btnClone && notify == win.BN_CLICKED {
				sel := int(win.SendMessage(listBox, win.LB_GETCURSEL, 0, 0))
				if sel >= 0 && sel < len(ENTITIES) {
					// Deep copy KeyValues map
					orig := ENTITIES[sel]
					clonedKV := make(map[string]string, len(orig.KeyValues))
					maps.Copy(clonedKV, orig.KeyValues)
					newEntity := Entity{KeyValues: clonedKV}

					// Append to ENTITIES
					ENTITIES = append(ENTITIES, newEntity)
					name := newEntity.KeyValues["classname"]
					if name == "" {
						name = "<no classname>"
					}

					// Update listbox
					blUpdatingListbox = true
					setRedraw(listBox, false)
					idx := int(win.SendMessage(listBox, win.LB_ADDSTRING, 0, uintptr(unsafe.Pointer(utf16Ptr(name)))))
					win.SendMessage(listBox, win.LB_SETCURSEL, uintptr(idx), 0)
					win.SendMessage(listBox, win.LB_SETTOPINDEX, uintptr(idx), 0)
					setRedraw(listBox, true)
					blUpdatingListbox = false

					// Show cloned entity in textbox
					setWindowText(textBox, renderKeyValues(newEntity.KeyValues))

					// Persist asynchronously
					snap := snapshotEntities(ENTITIES)
					go func(hwnd win.HWND, ents []Entity, sel int) {
						err := saveEntities(strEntityFile, ents)
						status := uintptr(0)
						if err != nil {
							status = 1
						}
						win.PostMessage(hwnd, WM_SAVE_COMPLETE, status, uintptr(sel))
					}(hwnd, snap, idx)
				}
			}

			// Delete: guarded UI mutation + async persist
			if src == btnDelete && notify == win.BN_CLICKED {
				sel := int(win.SendMessage(listBox, win.LB_GETCURSEL, 0, 0))
				if sel >= 0 && sel < len(ENTITIES) {
					ENTITIES = append(ENTITIES[:sel], ENTITIES[sel+1:]...)

					blUpdatingListbox = true
					setRedraw(listBox, false)
					win.SendMessage(listBox, win.LB_DELETESTRING, uintptr(sel), 0)
					if sel >= len(ENTITIES) {
						sel = len(ENTITIES) - 1
					}
					if sel >= 0 {
						win.SendMessage(listBox, win.LB_SETCURSEL, uintptr(sel), 0)
						win.SendMessage(listBox, win.LB_SETTOPINDEX, uintptr(sel), 0)
					}
					setRedraw(listBox, true)
					blUpdatingListbox = false

					if sel >= 0 {
						setWindowText(textBox, renderKeyValues(ENTITIES[sel].KeyValues))
					} else {
						setWindowText(textBox, "")
					}

					snap := snapshotEntities(ENTITIES)
					go func(hwnd win.HWND, ents []Entity, sel int) {
						err := saveEntities(strEntityFile, ents)
						status := uintptr(0)
						if err != nil {
							// logErr(err)
							status = 1
						}
						win.PostMessage(hwnd, WM_SAVE_COMPLETE, status, uintptr(sel))
					}(hwnd, snap, sel)
				}
			}
		}

	case WM_SAVE_COMPLETE:
		{
			status := wParam
			sel := int(lParam)
			if status == 0 && sel >= 0 && sel < len(ENTITIES) {
				// refresh UI...
				name := ENTITIES[sel].KeyValues["classname"]
				if name == "" {
					name = "<no classname>"
				}
				blUpdatingListbox = true
				top := int(win.SendMessage(listBox, win.LB_GETTOPINDEX, 0, 0))
				setRedraw(listBox, false)
				win.SendMessage(listBox, win.LB_DELETESTRING, uintptr(sel), 0)
				win.SendMessage(listBox, win.LB_INSERTSTRING, uintptr(sel), uintptr(unsafe.Pointer(utf16Ptr(name))))
				win.SendMessage(listBox, win.LB_SETCURSEL, uintptr(sel), 0)
				if top >= 0 {
					win.SendMessage(listBox, win.LB_SETTOPINDEX, uintptr(top), 0)
				}
				setRedraw(listBox, true)
				blUpdatingListbox = false

				// If Save button initiated this, close via WM_CLOSE
				if CloseMode == closeSilent {
					win.PostMessage(hwnd, win.WM_CLOSE, 0, 0)
				}
			}
		}

		// Centralized close logic — runs exactly once
	case win.WM_CLOSE:
		{
			mode := CloseMode
			if mode == closeNone {
				mode = closePrompt
			}

			if mode == closeSilent {
				// Save button path — already saved, apply silently
				RipJSON(strEntityFile, true, true) // -u
			} else { // Prompt path
				// Ask the user in a GUI dialog
				res := win.MessageBox(hwnd,
					syscall.StringToUTF16Ptr("Apply changes to BSP?"),
					syscall.StringToUTF16Ptr("Confirm Apply"),
					win.MB_YESNOCANCEL|win.MB_ICONQUESTION)

				switch res {
				case win.IDYES:
					_ = saveEntities(strEntityFile, ENTITIES)
					RipJSON(strEntityFile, true, true) // apply silently after GUI confirm
				case win.IDNO:
					// Discard changes — do nothing
				case win.IDCANCEL:
					return 0 // abort close entirely
				}
			}

			_ = os.Remove(strEntityFile)
			setRedraw(hwnd, false)
			win.SetFocus(0)

			CloseMode = closeNone
			win.DestroyWindow(hwnd)
			return 0
		}

	case win.WM_DESTROY:
		{
			// No RipJSON here — it already ran in WM_CLOSE
			win.UnregisterClass(pClassName)
			win.PostQuitMessage(0)
			return 0
		}
	}

	return win.DefWindowProc(hwnd, msg, wParam, lParam)
}

func utf16Ptr(s string) *uint16 {
	p, _ := syscall.UTF16PtrFromString(s)
	return p
}

func setRedraw(hwnd win.HWND, enable bool) {
	if enable {
		win.SendMessage(hwnd, win.WM_SETREDRAW, 1, 0)
		win.InvalidateRect(hwnd, nil, true)
	} else {
		win.SendMessage(hwnd, win.WM_SETREDRAW, 0, 0)
	}
}

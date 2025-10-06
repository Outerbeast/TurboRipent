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
	hInstance  win.HINSTANCE
	editor     *EditorUI
	registerUI sync.Once
	pClassName *uint16
	CloseMode  = closeNone
)

var (
	dllUser32                = syscall.NewLazyDLL("user32.dll")
	procGetWindowTextW       = dllUser32.NewProc("GetWindowTextW")
	procGetWindowTextLengthW = dllUser32.NewProc("GetWindowTextLengthW")
	procSetWindowTextW       = dllUser32.NewProc("SetWindowTextW")
)

var (
	ENTITIES          []Entity
	FILTERED_IDXS     []int
	strEntityFile     string
	blUpdatingListbox bool
	prevSel           int = -1
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

type EditorUI struct {
	List   ListBox
	Text   TextBox
	Filter TextBox
	Create Button
	Clone  Button
	Delete Button
	Save   Button
}

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

// Refreshes the listbox entry for the currently selected entity
func refreshSelectedEntityListbox() {

	sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))
	if sel < 0 || sel >= len(ENTITIES) {
		return
	}

	className := ENTITIES[sel].KeyValues["classname"]

	if className == "" {
		className = "<no classname>"
	}

	win.SendMessage(editor.List.hwnd, win.LB_DELETESTRING, uintptr(sel), 0)
	win.SendMessage(editor.List.hwnd, win.LB_INSERTSTRING, uintptr(sel), uintptr(unsafe.Pointer(wtfPointer(className))))
	win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, uintptr(sel), 0)
}

// Snapshot to avoid data races during async save
func snapshotEntities(ENTITIES []Entity) []Entity {

	OUT := make([]Entity, len(ENTITIES))

	for i := range ENTITIES {

		OUT[i].KeyValues = make(map[string]string, len(ENTITIES[i].KeyValues))

		maps.Copy(OUT[i].KeyValues, ENTITIES[i].KeyValues)
	}

	return OUT
}

func saveEntities(path string, ENTITIES []Entity) error {

	data, err := json.MarshalIndent(ENTITIES, "", "  ")

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

func setRedraw(hwnd win.HWND, enable bool) {

	if enable {
		win.SendMessage(hwnd, win.WM_SETREDRAW, 1, 0)
		win.InvalidateRect(hwnd, nil, true)
	} else {
		win.SendMessage(hwnd, win.WM_SETREDRAW, 0, 0)
	}
}

func applyEntityFilter(filter string) {

	filter = strings.ToLower(strings.TrimSpace(filter))
	blUpdatingListbox = true
	defer func() { blUpdatingListbox = false }()

	// Clear and rebuild
	win.SendMessage(editor.List.hwnd, win.LB_RESETCONTENT, 0, 0)
	FILTERED_IDXS = FILTERED_IDXS[:0]

	if filter == "" {

		for i, name := range classnamesFromEntities(ENTITIES) {

			editor.List.AddString(name)
			FILTERED_IDXS = append(FILTERED_IDXS, i)
		}
	} else {

		for i, ent := range ENTITIES {

			// If classname can be missing, guard it
			class := ent.KeyValues["classname"]
			for k, v := range ent.KeyValues {

				if strings.Contains(strings.ToLower(k), filter) || strings.Contains(strings.ToLower(v), filter) {

					editor.List.AddString(class)
					FILTERED_IDXS = append(FILTERED_IDXS, i)

					break
				}
			}
		}
	}

	// Update selection and editor text consistently
	if len(FILTERED_IDXS) > 0 {

		win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, 0, 0)
		idx := FILTERED_IDXS[0]
		editor.Text.SetText(renderKeyValues(ENTITIES[idx].KeyValues))
		prevSel = idx
	} else {
		// No match: clear selection and text
		win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, uintptr(^uint32(0)), 0) // LB_ERR
		editor.Text.SetText("")
		prevSel = -1
	}
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
	hideConsole()

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
			LoudPanic("RegisterClassEx failed:", syscall.GetLastError())
		}
	})

	wndEditor := CreateWindow(WindowSpec{
		ClassName: pClassName,
		Title:     AppName + " Editor - " + filepath.Base(chosenBSP),
		Style:     win.WS_OVERLAPPEDWINDOW,
		X:         win.CW_USEDEFAULT,
		Y:         win.CW_USEDEFAULT,
		W:         windowWidth,
		H:         windowHeight,
		HInstance: hInstance,
	})

	win.ShowWindow(wndEditor, win.SW_SHOWDEFAULT)
	win.UpdateWindow(wndEditor)

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
			editor = &EditorUI{
				List:   NewListBox(hwnd, 10, 10, 300, 400),
				Text:   NewTextBox(hwnd, 320, 10, 450, 360),
				Filter: NewTextBox(hwnd, 10, 420, 300, 25),
				Create: NewButton(hwnd, "Create", 0, 0, 80, 30, onCreate),
				Clone:  NewButton(hwnd, "Clone", 0, 0, 80, 30, onClone),
				Delete: NewButton(hwnd, "Delete", 0, 0, 80, 30, onDelete),
				Save:   NewButton(hwnd, "Save", 0, 0, 80, 30, onSave),
			}

			style := uint32(win.GetWindowLong(editor.Filter.hwnd, win.GWL_STYLE))
			style &^= win.WS_VSCROLL | win.ES_AUTOVSCROLL
			win.SetWindowLong(editor.Filter.hwnd, win.GWL_STYLE, int32(style))
			win.SetWindowPos(editor.Filter.hwnd,
				0,
				0, 0, 0, 0,
				win.SWP_NOMOVE|win.SWP_NOSIZE|win.SWP_NOZORDER|win.SWP_FRAMECHANGED,
			)

			// Load entities
			var err error
			ENTITIES, err = loadEntities(strEntityFile)

			if err != nil {
				editor.Text.SetText(fmt.Sprintf("Error loading entities.json:\r\n%v", err))
				return 0
			}
			// Populate listbox
			setRedraw(editor.List.hwnd, false)

			for i, name := range classnamesFromEntities(ENTITIES) {

				editor.List.AddString(name)
				FILTERED_IDXS = append(FILTERED_IDXS, i) // initial 1:1 mapping
			}

			setRedraw(editor.List.hwnd, true)

			if len(ENTITIES) > 0 {

				win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, 0, 0)
				editor.Text.SetText(renderKeyValues(ENTITIES[0].KeyValues))
				prevSel = 0
			}
		}

	case win.WM_GETMINMAXINFO:
		{
			mmi := (*win.MINMAXINFO)(unsafe.Pointer(lParam))
			//!-TODO-!: Replace all of these magic numbers, which will take forever
			margin, buttonWidth, buttonSpacing, numButtons := 10, 80, 10, 4

			// Minimum width to fit listbox + textbox + buttons
			minTextBoxWidth := buttonWidth*numButtons + buttonSpacing*(numButtons-1)
			minWidth := 350 + margin*3 + minTextBoxWidth

			// Minimum height to fit listbox + buttons
			minHeight := 200 + margin*2 + 40 // list height + margins + button height + spacing

			mmi.PtMinTrackSize.X = int32(minWidth)
			mmi.PtMinTrackSize.Y = int32(minHeight)

			return 0
		}

	case win.WM_SIZE:
		{
			width, height := int(win.LOWORD(uint32(lParam))), int(win.HIWORD(uint32(lParam)))
			margin, buttonWidth, buttonHeight, buttonSpacing, filterHeight := 10, 80, 30, 10, 25

			// Height for listbox and textbox so they match
			listHeight := height - (margin*2 + buttonHeight + buttonSpacing)

			// ListBox on the left
			win.MoveWindow(editor.List.hwnd, int32(margin), int32(margin), 300, int32(listHeight)+10, true)

			// TextBox to the right of listbox
			textX := 300 + margin*2
			textWidth := width - textX - margin
			win.MoveWindow(editor.Text.hwnd, int32(textX), int32(margin), int32(textWidth), int32(listHeight), true)

			// Buttons row Y position
			btnY := margin + listHeight + buttonSpacing

			// Create aligned to left edge of textbox
			createX := textX
			win.MoveWindow(editor.Create.hwnd, int32(createX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Clone to the right of Create
			cloneX := createX + buttonWidth + buttonSpacing
			win.MoveWindow(editor.Clone.hwnd, int32(cloneX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Delete to the right of Clone
			deleteX := cloneX + buttonWidth + buttonSpacing
			win.MoveWindow(editor.Delete.hwnd, int32(deleteX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)

			// Save anchored to far right
			saveX := width - margin - buttonWidth
			win.MoveWindow(editor.Save.hwnd, int32(saveX), int32(btnY), int32(buttonWidth), int32(buttonHeight), true)
			win.MoveWindow(editor.List.hwnd, int32(margin), int32(margin), 300, int32(listHeight), true)
			win.MoveWindow(editor.Filter.hwnd, int32(margin), int32(margin+listHeight+margin), 300, int32(filterHeight), true)
		}

	case win.WM_COMMAND:
		{
			src := win.HWND(lParam)
			notify := win.HIWORD(uint32(wParam))

			switch {
			// --- Listbox selection change ---
			case src == editor.List.hwnd && notify == win.LBN_SELCHANGE && !blUpdatingListbox:
				{
					sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))
					if sel >= 0 && sel < len(FILTERED_IDXS) {
						idx := FILTERED_IDXS[sel]
						if prevSel != -1 && prevSel < len(ENTITIES) {
							ENTITIES[prevSel].KeyValues = parseKeyValues(editor.Text.Text())
						}
						editor.Text.SetText(renderKeyValues(ENTITIES[idx].KeyValues))
						prevSel = idx
					}
				}

			// --- Textbox edits ---
			case src == editor.Text.hwnd && notify == win.EN_CHANGE:
				{
					sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))
					if sel >= 0 && sel < len(FILTERED_IDXS) {
						idx := FILTERED_IDXS[sel]
						ENTITIES[idx].KeyValues = parseKeyValues(editor.Text.Text())
						refreshSelectedEntityListbox()
					}
				}

			// --- Filter box edits ---
			case src == editor.Filter.hwnd && notify == win.EN_CHANGE:
				applyEntityFilter(editor.Filter.Text())

			// --- Buttons ---
			case src == editor.Create.hwnd:
				editor.Create.HandleCommand(notify, hwnd)
			case src == editor.Clone.hwnd:
				editor.Clone.HandleCommand(notify, hwnd)
			case src == editor.Delete.hwnd:
				editor.Delete.HandleCommand(notify, hwnd)
			case src == editor.Save.hwnd:
				editor.Save.HandleCommand(notify, hwnd)
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
			} else {

				switch MessageBox("Confirm changes", "Apply changes to BSP?", win.MB_YESNOCANCEL|win.MB_ICONQUESTION) {

				case win.IDYES:
					{
						_ = saveEntities(strEntityFile, ENTITIES)
						RipJSON(strEntityFile, true, true) // apply silently after GUI confirm
					}

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

	// WM_DESTROY message remains the same
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

func onCreate(hwnd win.HWND) {

	var kv map[string]string

	if len(mapDefaultEntityTemplate) > 0 {
		kv = make(map[string]string, len(mapDefaultEntityTemplate))
		maps.Copy(kv, mapDefaultEntityTemplate)
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
	setRedraw(editor.List.hwnd, false)
	idx := editor.List.AddString(name)
	win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, uintptr(idx), 0)
	win.SendMessage(editor.List.hwnd, win.LB_SETTOPINDEX, uintptr(idx), 0)
	setRedraw(editor.List.hwnd, true)
	blUpdatingListbox = false
	prevSel = idx
	editor.Text.SetText(renderKeyValues(newEntity.KeyValues))
	snap := snapshotEntities(ENTITIES)

	go func() {
		_ = saveEntities(strEntityFile, snap)
		win.PostMessage(hwnd, WM_SAVE_COMPLETE, 0, uintptr(idx))
	}()
}

func onClone(hwnd win.HWND) {

	sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))

	if sel < 0 || sel >= len(ENTITIES) {
		return
	}

	orig := ENTITIES[sel]
	clonedKV := make(map[string]string, len(orig.KeyValues))
	maps.Copy(clonedKV, orig.KeyValues)
	newEntity := Entity{KeyValues: clonedKV}
	ENTITIES = append(ENTITIES, newEntity)

	name := newEntity.KeyValues["classname"]

	if name == "" {
		name = "<no classname>"
	}

	blUpdatingListbox = true
	setRedraw(editor.List.hwnd, false)
	idx := editor.List.AddString(name)
	win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, uintptr(idx), 0)
	win.SendMessage(editor.List.hwnd, win.LB_SETTOPINDEX, uintptr(idx), 0)
	setRedraw(editor.List.hwnd, true)
	blUpdatingListbox = false
	prevSel = idx
	editor.Text.SetText(renderKeyValues(newEntity.KeyValues))

	snap := snapshotEntities(ENTITIES)

	go func() {
		_ = saveEntities(strEntityFile, snap)
		win.PostMessage(hwnd, WM_SAVE_COMPLETE, 0, uintptr(idx))
	}()
}

func onDelete(hwnd win.HWND) {

	sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))

	if sel < 0 || sel >= len(ENTITIES) {
		return
	}

	ENTITIES = append(ENTITIES[:sel], ENTITIES[sel+1:]...)
	blUpdatingListbox = true
	setRedraw(editor.List.hwnd, false)
	win.SendMessage(editor.List.hwnd, win.LB_DELETESTRING, uintptr(sel), 0)

	if sel < len(ENTITIES) {

		win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, uintptr(sel), 0)
		win.SendMessage(editor.List.hwnd, win.LB_SETTOPINDEX, uintptr(sel), 0)
		editor.Text.SetText(renderKeyValues(ENTITIES[sel].KeyValues))
	} else {

		win.SendMessage(editor.List.hwnd, win.LB_SETCURSEL, ^uintptr(0), 0)
		win.SendMessage(editor.List.hwnd, win.LB_SETTOPINDEX, ^uintptr(0), 0)
		prevSel = -1
		editor.Text.SetText("")
	}

	setRedraw(editor.List.hwnd, true)
	blUpdatingListbox = false
	snap := snapshotEntities(ENTITIES)

	go func() {
		_ = saveEntities(strEntityFile, snap)
		win.PostMessage(hwnd, WM_SAVE_COMPLETE, 0, uintptr(sel))
	}()
}

func onSave(hwnd win.HWND) {

	CloseMode = closeSilent
	sel := int(win.SendMessage(editor.List.hwnd, win.LB_GETCURSEL, 0, 0))

	if sel < 0 || sel >= len(ENTITIES) {
		return
	}

	ENTITIES[sel].KeyValues = parseKeyValues(editor.Text.Text())
	snap := snapshotEntities(ENTITIES)

	go func() {
		_ = saveEntities(strEntityFile, snap)
		win.PostMessage(hwnd, WM_SAVE_COMPLETE, 0, uintptr(sel))
	}()
}

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
	"unsafe"

	"github.com/lxn/win"
)

type ListBox struct{ hwnd win.HWND }
type TextBox struct{ hwnd win.HWND }
type Label struct{ hwnd win.HWND }
type Button struct {
	hwnd    win.HWND
	onClick func(win.HWND)
}

type WindowSpec struct {
	ClassName  *uint16
	Title      string
	X, Y, W, H int32
	Style      uint32
	HInstance  win.HINSTANCE
}

func CreateWindow(spec WindowSpec) win.HWND {

	return win.CreateWindowEx(
		0,
		spec.ClassName,
		wtfPointer(spec.Title),
		spec.Style,
		spec.X, spec.Y,
		spec.W, spec.H,
		0, 0, spec.HInstance, nil,
	)
}

func NewListBox(parent win.HWND, x, y, w, h int32) ListBox {

	hwnd := win.CreateWindowEx(
		0, wtfPointer("LISTBOX"), nil,
		win.WS_CHILD|win.WS_VISIBLE|win.WS_BORDER|win.WS_VSCROLL|win.LBS_NOTIFY,
		x, y, w, h, parent, 0, hInstance, nil,
	)

	return ListBox{hwnd}
}

func NewButton(parent win.HWND, label string, x, y, w, h int32, onClick func(win.HWND)) Button {

	hwnd := win.CreateWindowEx(
		0, wtfPointer("BUTTON"), wtfPointer(label),
		win.WS_CHILD|win.WS_VISIBLE,
		x, y, w, h, parent, 0, hInstance, nil,
	)

	return Button{hwnd: hwnd, onClick: onClick}
}

func (b Button) HandleCommand(notify uint16, parent win.HWND) {

	if notify == win.BN_CLICKED && b.onClick != nil {
		b.onClick(parent)
	}
}

func NewTextBox(parent win.HWND, x, y, w, h int32) TextBox {

	hwnd := win.CreateWindowEx(

		0, wtfPointer("EDIT"), nil,
		win.WS_CHILD|win.WS_VISIBLE|win.WS_BORDER|win.ES_MULTILINE|win.ES_AUTOVSCROLL|win.WS_VSCROLL,
		x, y, w, h, parent, 0, hInstance, nil,
	)

	return TextBox{hwnd}
}

func (tb TextBox) Text() string {
	return getWindowText(tb.hwnd)
}

func (lb ListBox) AddString(s string) int {

	return int(win.SendMessage(lb.hwnd, win.LB_ADDSTRING, 0,
		uintptr(unsafe.Pointer(wtfPointer(s)))))
}

func (tb TextBox) SetText(s string) {
	setWindowText(tb.hwnd, s)
}

func MessageBox(title, message string, flags uint32) int {

	return int(win.MessageBox(0, wtfPointer(message), wtfPointer(title), flags))
}

// Unused
func NewLabel(parent win.HWND, text string, x, y, w, h int32) Label {

	hwnd := win.CreateWindowEx(
		0,
		wtfPointer("STATIC"),
		wtfPointer(text),
		win.WS_CHILD|win.WS_VISIBLE,
		x, y, w, h,
		parent,
		0,
		hInstance,
		nil,
	)

	return Label{hwnd: hwnd}
}

func (l Label) SetText(s string) {
	setWindowText(l.hwnd, s)
}

func (l Label) Text() string {
	return getWindowText(l.hwnd)
}

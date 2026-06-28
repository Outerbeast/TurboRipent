/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.0

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
use std::
{
    path::
    {
        Path,
        PathBuf
    }
};

use native_windows_gui::
{
    Button, 
    ListBox, 
    Monitor, 
    TextBox, 
    TextBoxFlags,
    TextInput,
    TextInputFlags,
    Window, 
    WindowFlags, 
    ListBoxFlags, 
    ButtonFlags, 
    NwgError
};

use crate::
{
    APPNAME,
    alloc_leaked
};

// (w/h)
const WINDOW_SIZE: (i32, i32) = ( 800, 505 );
const BUTTON_SIZE: (i32, i32) = ( 80, 30 );
const CLASSNAME_LIST_SIZE: (i32, i32) = ( 290, 460 );
const PROPERTIES_BOX_SIZE: (i32, i32) = ( 490, 450 );
const FILTER_BOX_SIZE: (i32, i32) = ( CLASSNAME_LIST_SIZE.0, 25 );

#[derive( Default )]
pub struct EditorWindow
{
    pub window: Window,
    pub list: ListBox<String>,
    pub text: TextBox,
    pub filter: TextInput,
    pub btn_create: Button,
    pub btn_clone: Button,
    pub btn_delete: Button,
    pub btn_save: Button,
    pub file_path: PathBuf,
}

impl EditorWindow
{
    pub fn build(file_path: &Path) -> Result<&'static mut Self, NwgError>
    {
        let mut app = Self
        {
            file_path: file_path.to_path_buf(),
            ..Default::default()
        };

        Window::builder()
            .size( WINDOW_SIZE )
            .position(
            {
                let center_x = ( Monitor::width() - WINDOW_SIZE.0 ) / 2;
                let center_y = ( Monitor::height() - WINDOW_SIZE.1 ) / 2;
                ( center_x, center_y )
            })
            .title( &format!( "{APPNAME} Editor - {}", file_path.file_name().unwrap_or_default().to_string_lossy() ) )
            .flags( WindowFlags::WINDOW | WindowFlags::VISIBLE | WindowFlags::MINIMIZE_BOX )
        .build( &mut app.window )?;
        // Entity classname ListBox on the left
        ListBox::builder()
            .parent( &app.window )
            .position( ( 10, 10 ) )
            .size( CLASSNAME_LIST_SIZE )
            .flags( ListBoxFlags::VISIBLE )
            .collection( vec![] )
        .build( &mut app.list )?;
        // Entity Properties TextBox to the right of listbox
        TextBox::builder()
            .parent( &app.window )
            .position( ( 310, 10 ) )
            .size( PROPERTIES_BOX_SIZE )
            .flags( TextBoxFlags::VISIBLE | TextBoxFlags::VSCROLL )
            .text( "" )
        .build( &mut app.text )?;
        // Filter TextInput below listbox
        TextInput::builder()
            .parent( &app.window )
            .position( ( 10, 470 ) )
            .size( FILTER_BOX_SIZE )
            .flags( TextInputFlags::VISIBLE | TextInputFlags::AUTO_SCROLL )
            .text( "" )
        .build( &mut app.filter )?;
        // Buttons row
        // Create button - aligned to left edge of textbox (x=320)
        Button::builder()
            .parent( &app.window )
            .position( ( 310, 465 ) )
            .size( BUTTON_SIZE )
            .flags( ButtonFlags::VISIBLE )
            .text( "Create" )
        .build( &mut app.btn_create )?;
        // Clone button
        Button::builder()
            .parent( &app.window )
            .position( ( 400, 465 ) )
            .size( BUTTON_SIZE )
            .flags( ButtonFlags::VISIBLE )
            .text( "Clone" )
        .build( &mut app.btn_clone )?;
        // Delete button
        Button::builder()
            .parent( &app.window )
            .position( ( 490, 465 ) )
            .size( BUTTON_SIZE )
            .flags( ButtonFlags::VISIBLE )
            .text( "Delete" )
        .build( &mut app.btn_delete )?;
        // Save button - anchored to far right
        Button::builder()
            .parent( &app.window )
            .position( ( 700, 465 ) )
            .size( BUTTON_SIZE )
            .flags( ButtonFlags::VISIBLE )
            .text( "Save" )
        .build( &mut app.btn_save )?;

        Ok( alloc_leaked!( app ) )
    }
}
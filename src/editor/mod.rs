/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.1.0

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
pub mod view;
pub mod controller;

use std::path::
{
    Path,
    PathBuf
};

use anyhow::
{
    Result,
    bail
};

use super::
{
    cli::get_prompt_input,
    utils::
    {
        hide_terminal,
        show_terminal
    },
    bsp::ent::EntityDictionary
};

use crate::prelude::*;

impl EntityDictionary
{   /// This is used by the editor to construct entity dictionary from key=value pairs
    pub fn parse_keyvalues(s: &str) -> Self
    {
        let mut kvs = Self::default();

        for line in s.lines()
        {
            let line = line.trim();
            if line.is_empty()
            {
                continue;
            }

            if let Some( eq_pos ) = line.find( '=' )
            {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos + 1..].trim().to_string();
                kvs.insert( key, val );
            }
        }

        kvs
    }
    /// This is used by the editor to render entity dictionary into a key=value line
    /// Returns None if the dictionary is empty
    pub fn render_keyvalues(&self) -> Option<String>
    {
        if self.is_empty()
        {
            return None;
        }

        let mut keys: Vec<_> = self.keys().collect();
        keys.sort();

        let body = keys
            .iter()
            .map( |k| format!( "{k}={}", self[*k] ) )
            .collect::<Vec<_>>()
        .join( "\r\n" );

        Some( body )
    }
}
/// Launch the application with the given to BSP or ENT
pub fn launch(chosen_path: impl AsRef<Path>) -> Result<()>
{
    let chosen_path = chosen_path.as_ref();
    let file_path =
    if chosen_path.to_string_lossy().is_empty() 
    || !chosen_path.has_extension( &[EXT_BSP, EXT_ENT, EXT_POINT_ENT, EXT_BRUSH_ENT] )
    {
        let path_str =
        get_prompt_input( "Drag a BSP or ENT file you want to edit (enter 'x' to cancel):" );

        if path_str.is_empty() || path_str == "x"
        {
            bail!( "User cancelled." );
        }
        
        PathBuf::from( path_str )
    }
    else
    {
        PathBuf::from( chosen_path )
    };

    println!( "Opening: {file_path:?}" );
    hide_terminal();
    let entity_dicts = EntityDictionary::load_entities( &file_path )?;
    // Launch the GUI
    let gui = view::EditorWindow::new( &file_path )?;
    controller::EditorController::new( gui, entity_dicts ).register( gui );
    // Hide the GUI window so it doesn't obscure the console
    #[cfg( target_os = "windows" )]
    if let Some( hwnd ) = gui.window.handle.hwnd()
    {
        unsafe extern "system"
        {
            fn ShowWindow(hwnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        }

        unsafe { ShowWindow( hwnd as *mut std::ffi::c_void, 0 ); }// SW_HIDE = 0
    }

    show_terminal();

    Ok( () )
}

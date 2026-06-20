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
pub mod view;
pub mod controller;

use std::
{
    fs,
    path::
    {
        Path,
        PathBuf
    }
};

use anyhow::
{
    Result,
    bail
};

use native_windows_gui::
{
    dispatch_thread_events,
    init
};

use super::
{
    menu::get_prompt_input,
    utils::
    {
        hide_terminal,
        show_terminal
    },
    bsp::
    {
        ent::
        {
            Dictionary,
            parse_entity_blocks,
            EXT_BSP, 
            EXT_BRUSH_ENT,
            EXT_ENT,
            EXT_POINT_ENT
        },
        BspFile,
        LumpIdx
    }
};
// Launch the application with the given to BSP or ENT
pub fn launch(chosen_path: impl AsRef<Path>) -> Result<()>
{
    let chosen_path = chosen_path.as_ref();
    let file_path =
    if chosen_path.to_string_lossy().is_empty() || !matches!( chosen_path.extension()
        .and_then( |ext| ext.to_str() ), 
        Some( EXT_BSP ) | Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) )
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

    let entity_dicts=
    match file_path.extension().and_then( |ext| ext.to_str() )
    {
        Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>// ENT file editing
        {
            parse_entity_blocks( &fs::read_to_string( &file_path )? )
                .into_iter()
                .map( |(_, dict)| dict )
            .collect()
        }
        
        Some( EXT_BSP ) =>
        {
            let bsp = BspFile::load( &file_path )?;
            let lump_text = str::from_utf8( bsp.slice_lump( LumpIdx::Entities ) )?;
            parse_entity_blocks( lump_text ).into_iter().map( |(_, dict)| dict ).collect()
        }

        _ => bail!( format!( "Unsupported file type '{:?}'", file_path.extension().unwrap_or_default() ) )
        
    };

    controller::ENTITIES.with( |ent| *ent.borrow_mut() = entity_dicts );
    // Launch the GUI
    init()?;
    let gui = view::EditorWindow::build( &file_path )?;
    controller::setup_event_handlers( gui );
    // Populate listbox after GUI is built
    controller::populate_listbox( gui );
    dispatch_thread_events();
    // Hide the GUI window so it doesn't obscure the console
    #[cfg(target_os = "windows")]
    if let Some( hwnd ) = gui.window.handle.hwnd()
    {
        unsafe extern "system"
        {
            fn ShowWindow(hwnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        }

        unsafe { ShowWindow( hwnd as *mut std::ffi::c_void, 0 ); } // SW_HIDE = 0
    }

    show_terminal();

    Ok( () )
}
